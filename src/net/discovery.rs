//! Multicast discovery: the announcer thread, the listener thread, and the
//! registry of peers both feed.
//!
//! Flow per the spec: we multicast our [`DeviceInfo`] with `announce: true`;
//! whoever hears it POSTs their info to our `/register` (handled in
//! [`super::server`]). Symmetrically, when we hear an announce we POST our
//! info to the announcer — and if that TCP path fails, fall back to a
//! multicast reply with `announce: false`. Every valid packet whose
//! fingerprint isn't ours upserts the registry either way.

use super::protocol::{self, DeviceInfo};
use super::{NetShared, WakeReason};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Peers silent longer than this are dropped from snapshots.
const PEER_TTL: Duration = Duration::from_secs(120);
/// Announce datagrams are small JSON; anything bigger is not for us.
const MAX_PACKET: usize = 64 * 1024;
/// Most peers kept at once. A network has a handful; a flood of made-up
/// fingerprints would otherwise grow the registry — and the radar — without end.
const MAX_PEERS: usize = 64;
/// One register-reply per peer per this long. A reply costs a thread and an
/// HTTP round trip, and an announcer that repeats itself (the spec's own client
/// bursts at startup) needs only the first.
const REPLY_EVERY: Duration = Duration::from_secs(5);
/// Fingerprints tracked for [`REPLY_EVERY`] before the stale ones are dropped.
const MAX_REPLY_TRACKED: usize = 256;

#[derive(Clone, Debug)]
pub struct Peer {
    pub info: DeviceInfo,
    pub ip: IpAddr,
    /// From the announce/register body; the port we dial for transfers.
    pub port: u16,
    pub last_seen: Instant,
    /// Typed in by hand (see [`super::manual`]). Nothing refreshes such a peer,
    /// so it is exempt from [`PEER_TTL`] pruning.
    pub manual: bool,
}

impl Peer {
    /// URL scheme this peer serves ("http"/"https"), from its announce.
    pub fn scheme(&self) -> &str {
        self.info.protocol.as_deref().unwrap_or("http")
    }

    /// Base URL for the peer's REST endpoints. Built through [`SocketAddr`] so
    /// an IPv6 peer comes out bracketed (`https://[fe80::1]:53317`).
    pub fn base_url(&self) -> String {
        format!(
            "{}://{}",
            self.scheme(),
            SocketAddr::new(self.ip, self.port)
        )
    }
}

#[derive(Default)]
pub struct PeerRegistry {
    peers: Mutex<std::collections::HashMap<String, Peer>>,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert/refresh a peer keyed by fingerprint. Returns whether anything
    /// visible changed (new peer or changed identity), so callers can skip
    /// waking the UI for a pure keep-alive.
    pub fn upsert(&self, info: DeviceInfo, ip: IpAddr) -> bool {
        let Some(port) = info.port else {
            return false; // unreachable peer: nothing to dial
        };
        self.insert(info, ip, port, false)
    }

    /// Insert/refresh a hand-typed peer at the address that answered. That
    /// address wins over the announced port — it is the one we just proved
    /// reachable — and the entry becomes sticky.
    pub fn upsert_manual(&self, info: DeviceInfo, ip: IpAddr, port: u16) -> bool {
        self.insert(info, ip, port, true)
    }

    fn insert(&self, info: DeviceInfo, ip: IpAddr, port: u16, manual: bool) -> bool {
        // The one gate every stored peer passes: the names are a peer's to
        // choose and every screen draws them.
        let info = info.clamped();
        let mut peers = self.peers.lock().unwrap();
        if !peers.contains_key(&info.fingerprint) && peers.len() >= MAX_PEERS {
            // Full of peers, so make room the honest way before refusing: a
            // hand-typed one is nobody's to evict.
            peers.retain(|_, p| p.manual || p.last_seen.elapsed() < PEER_TTL);
            if peers.len() >= MAX_PEERS {
                log::warn!("{MAX_PEERS} peers already known; ignoring `{}`", info.alias);
                return false;
            }
        }
        let existing = peers.get(&info.fingerprint);
        let changed = match existing {
            Some(p) => p.info.alias != info.alias || p.ip != ip || p.port != port,
            None => true,
        };
        // Once sticky, always sticky: a later announce must not put a manual
        // peer back under the TTL it was added to escape.
        let manual = manual || existing.is_some_and(|p| p.manual);
        peers.insert(
            info.fingerprint.clone(),
            Peer {
                info,
                ip,
                port,
                last_seen: Instant::now(),
                manual,
            },
        );
        changed
    }

    /// Live peers sorted by alias; prunes expired entries as it goes.
    pub fn snapshot(&self) -> Vec<Peer> {
        let mut peers = self.peers.lock().unwrap();
        peers.retain(|_, p| p.manual || p.last_seen.elapsed() < PEER_TTL);
        let mut list: Vec<Peer> = peers.values().cloned().collect();
        list.sort_by(|a, b| a.info.alias.cmp(&b.info.alias).then(a.ip.cmp(&b.ip)));
        list
    }
}

pub enum AnnouncerMsg {
    ReAnnounce,
    Shutdown,
}

/// Multicast our presence: a burst at startup (multicast is lossy — the spec's
/// own client announces several times), then every `interval`, plus on demand.
pub fn spawn_announcer(
    shared: Arc<NetShared>,
    interval: Duration,
) -> std::io::Result<(JoinHandle<()>, mpsc::Sender<AnnouncerMsg>)> {
    let (tx, rx) = mpsc::channel::<AnnouncerMsg>();
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    // Deliver our own announces to local members too — that's how two
    // instances on one dev machine (or us + the official app) find each other.
    socket.set_multicast_loop_v4(true)?;

    let handle = std::thread::Builder::new()
        .name("announcer".into())
        .spawn(move || {
            for _ in 0..3 {
                announce_once(&socket, &shared);
                if rx
                    .recv_timeout(Duration::from_millis(500))
                    .is_ok_and(is_shutdown)
                {
                    return;
                }
                if shared.shutdown.load(Ordering::SeqCst) {
                    return;
                }
            }
            loop {
                match rx.recv_timeout(interval) {
                    Ok(AnnouncerMsg::Shutdown) => return,
                    Ok(AnnouncerMsg::ReAnnounce) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
                if shared.shutdown.load(Ordering::SeqCst) {
                    return;
                }
                announce_once(&socket, &shared);
            }
        })?;

    Ok((handle, tx))
}

fn is_shutdown(msg: AnnouncerMsg) -> bool {
    matches!(msg, AnnouncerMsg::Shutdown)
}

fn announce_once(socket: &UdpSocket, shared: &NetShared) {
    let mut me = shared.me.lock().unwrap().clone();
    me.announce = Some(true);
    let json = serde_json::to_string(&me).expect("DeviceInfo serializes");
    let dest = SocketAddr::from((protocol::MULTICAST_GROUP, protocol::MULTICAST_PORT));
    match socket.send_to(json.as_bytes(), dest) {
        Ok(_) => log::debug!("announced as `{}` on port {:?}", me.alias, me.port),
        // Non-fatal: wifi may be down right now; the next tick retries.
        Err(e) => log::warn!("announce failed: {e}"),
    }
}

/// Listen on the protocol's fixed multicast port. SO_REUSEADDR+SO_REUSEPORT
/// (socket2; std can't set them pre-bind) let us share the port with the
/// official LocalSend app on the same machine — multicast datagrams are
/// delivered to every joined socket, unlike unicast.
pub fn spawn_listener(shared: Arc<NetShared>) -> std::io::Result<JoinHandle<()>> {
    let socket = bind_multicast(protocol::MULTICAST_PORT)?;
    // Join if the network is up; else defer and let the loop retry, so the app
    // starts offline and discovery recovers when wifi appears.
    let mut joined = match join_multicast(&socket) {
        Ok(()) => true,
        Err(e) => {
            log::warn!("multicast join deferred (no network yet?): {e}");
            false
        }
    };

    std::thread::Builder::new()
        .name("discovery".into())
        .spawn(move || {
            let mut buf = vec![0u8; MAX_PACKET];
            // Local to this thread: nobody else decides who was answered when.
            let mut replied: std::collections::HashMap<String, Instant> =
                std::collections::HashMap::new();
            loop {
                if shared.shutdown.load(Ordering::SeqCst) {
                    return;
                }
                if !joined && join_multicast(&socket).is_ok() {
                    log::info!("multicast group joined");
                    joined = true;
                }
                let (len, src) = match socket.recv_from(&mut buf) {
                    Ok(r) => r,
                    // Read timeout: just a lap of the shutdown check.
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        continue;
                    }
                    Err(e) => {
                        log::error!("discovery socket error: {e}");
                        return;
                    }
                };
                let Ok(info) = serde_json::from_slice::<DeviceInfo>(&buf[..len]) else {
                    log::debug!("ignoring malformed datagram from {src}");
                    continue;
                };
                let our_fingerprint = shared.me.lock().unwrap().fingerprint.clone();
                if info.fingerprint == our_fingerprint {
                    continue; // our own loopback
                }
                let wants_reply = info.announce == Some(true);
                if shared.peers.upsert(info.clone(), src.ip()) {
                    log::info!("discovered `{}` at {}", info.alias, src.ip());
                    shared.wake.wake(WakeReason::Peers);
                }
                if wants_reply && due_a_reply(&mut replied, &info.fingerprint) {
                    reply_to_announce(&shared, info, src.ip());
                }
            }
        })
}

/// Whether `fingerprint` is owed a register-reply, remembering that it was.
/// Without this a peer announcing in a loop — or flooding on purpose — gets a
/// thread and an HTTP round trip per datagram.
fn due_a_reply(
    replied: &mut std::collections::HashMap<String, Instant>,
    fingerprint: &str,
) -> bool {
    if replied
        .get(fingerprint)
        .is_some_and(|at| at.elapsed() < REPLY_EVERY)
    {
        return false;
    }
    if replied.len() >= MAX_REPLY_TRACKED {
        replied.retain(|_, at| at.elapsed() < REPLY_EVERY);
        // Still full: that many strangers inside one interval is a flood, not a
        // network. Answering it is what would cost us.
        if replied.len() >= MAX_REPLY_TRACKED {
            return false;
        }
    }
    replied.insert(fingerprint.to_string(), Instant::now());
    true
}

fn bind_multicast(port: u16) -> std::io::Result<UdpSocket> {
    use super::context;
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
        .map_err(context("udp socket"))?;
    socket
        .set_reuse_address(true)
        .map_err(context("SO_REUSEADDR"))?;
    socket
        .set_reuse_port(true)
        .map_err(context("SO_REUSEPORT"))?;
    socket
        .bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())
        .map_err(context("udp bind"))?;
    let udp: UdpSocket = socket.into();
    udp.set_multicast_loop_v4(true)
        .map_err(context("IP_MULTICAST_LOOP"))?;
    udp.set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(context("udp read timeout"))?;
    Ok(udp)
}

/// Join the multicast group — the one step needing a live network (errors with
/// wifi down), so it's split from the bind and retried by the listener.
/// INADDR_ANY picks the default-route interface (the handheld's sole wifi NIC).
fn join_multicast(socket: &UdpSocket) -> std::io::Result<()> {
    socket.join_multicast_v4(&protocol::MULTICAST_GROUP, &Ipv4Addr::UNSPECIFIED)
}

/// Answer an announce: POST our info to the announcer's `/register` (a
/// short-lived thread; 2 s budget), falling back to a multicast reply with
/// `announce: false` if their HTTP endpoint can't be reached. The register
/// response carries their (fresher) info; upsert it best-effort.
fn reply_to_announce(shared: &Arc<NetShared>, their_info: DeviceInfo, their_ip: IpAddr) {
    let Some(their_port) = their_info.port else {
        return;
    };
    let shared = shared.clone();
    let spawned = std::thread::Builder::new()
        .name("register-reply".into())
        .spawn(move || {
            let me = { shared.me.lock().unwrap().clone() };
            let body = serde_json::to_string(&me).expect("DeviceInfo serializes");
            let scheme = their_info.protocol.as_deref().unwrap_or("http");
            let url = format!(
                "{scheme}://{their_ip}:{their_port}{}/register",
                protocol::API_PREFIX
            );
            match super::client::agent(Some(Duration::from_secs(2)))
                .post(&url)
                .content_type("application/json")
                .send(body.as_str())
            {
                Ok(mut resp) => {
                    let mut text = String::new();
                    let _ = resp
                        .body_mut()
                        .as_reader()
                        .take(MAX_PACKET as u64)
                        .read_to_string(&mut text);
                    if let Ok(info) = serde_json::from_str::<DeviceInfo>(&text) {
                        if info.fingerprint != me.fingerprint && shared.peers.upsert(info, their_ip)
                        {
                            shared.wake.wake(WakeReason::Peers);
                        }
                    }
                }
                Err(e) => {
                    log::debug!("register reply to {url} failed ({e}); multicast fallback");
                    let mut me = me;
                    me.announce = Some(false);
                    if let (Ok(json), Ok(socket)) = (
                        serde_json::to_string(&me),
                        UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)),
                    ) {
                        let _ = socket.set_multicast_loop_v4(true);
                        let dest =
                            SocketAddr::from((protocol::MULTICAST_GROUP, protocol::MULTICAST_PORT));
                        let _ = socket.send_to(json.as_bytes(), dest);
                    }
                }
            }
        });
    if let Err(e) = spawned {
        log::warn!("could not spawn register-reply thread: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(fingerprint: &str, port: Option<u16>) -> DeviceInfo {
        DeviceInfo {
            alias: "Peer".into(),
            version: protocol::PROTOCOL_VERSION.into(),
            device_model: None,
            device_type: None,
            fingerprint: fingerprint.into(),
            port,
            protocol: Some("http".into()),
            download: None,
            announce: None,
        }
    }

    /// Backdate a peer past the TTL, the way a device going quiet would.
    fn age_out(registry: &PeerRegistry, fingerprint: &str) {
        let mut peers = registry.peers.lock().unwrap();
        let peer = peers.get_mut(fingerprint).expect("peer was inserted");
        peer.last_seen = Instant::now() - PEER_TTL * 2;
    }

    #[test]
    fn silent_peers_expire_but_manual_ones_stay() {
        let registry = PeerRegistry::new();
        let ip = IpAddr::from([192, 168, 1, 5]);
        assert!(registry.upsert(info("heard", Some(53317)), ip));
        assert!(registry.upsert_manual(info("typed", None), ip, 53318));
        age_out(&registry, "heard");
        age_out(&registry, "typed");

        let live = registry.snapshot();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].info.fingerprint, "typed");
        // The dialed port, not the announce's missing one.
        assert_eq!(live[0].port, 53318);
    }

    #[test]
    fn an_announce_does_not_unstick_a_manual_peer() {
        let registry = PeerRegistry::new();
        let ip = IpAddr::from([192, 168, 1, 5]);
        registry.upsert_manual(info("typed", None), ip, 53317);
        registry.upsert(info("typed", Some(53317)), ip);
        age_out(&registry, "typed");
        assert_eq!(registry.snapshot().len(), 1);
    }

    /// Every stored peer passes the clamp, whichever way it arrived — the
    /// radar draws these names, and a peer picks them.
    #[test]
    fn a_stored_peers_name_is_clamped() {
        let registry = PeerRegistry::new();
        let ip = IpAddr::from([192, 168, 1, 5]);
        let mut flood = info("heard", Some(53317));
        flood.alias = "A".repeat(64 * 1024);
        flood.device_model = Some("M\nM".into());
        registry.upsert(flood, ip);

        let mut typed = info("typed", None);
        typed.alias = "B".repeat(1000);
        registry.upsert_manual(typed, ip, 53318);

        for peer in registry.snapshot() {
            assert!(
                peer.info.alias.chars().count() <= 48,
                "{}",
                peer.info.alias.len()
            );
            assert!(!peer.info.alias.contains('\n'));
        }
        let heard = registry
            .snapshot()
            .into_iter()
            .find(|p| p.info.fingerprint == "heard")
            .expect("peer was inserted");
        assert_eq!(heard.info.device_model.as_deref(), Some("M M"));
    }

    /// A peer that repeats itself gets one reply per interval, and a flood of
    /// strangers gets none — a reply is a thread and an HTTP round trip.
    #[test]
    fn replies_are_owed_once_per_interval() {
        let mut replied = std::collections::HashMap::new();
        assert!(due_a_reply(&mut replied, "peer"));
        assert!(!due_a_reply(&mut replied, "peer"), "answered twice");
        assert!(due_a_reply(&mut replied, "another"));

        // Its turn comes round again once the interval has passed.
        replied.insert("peer".to_string(), Instant::now() - REPLY_EVERY * 2);
        assert!(due_a_reply(&mut replied, "peer"));

        for i in 0..MAX_REPLY_TRACKED {
            due_a_reply(&mut replied, &format!("flood{i}"));
        }
        assert!(replied.len() <= MAX_REPLY_TRACKED, "{}", replied.len());
        assert!(!due_a_reply(&mut replied, "one-more-stranger"));
    }

    /// The registry is bounded, and a hand-typed peer is never what gets
    /// dropped to make room.
    #[test]
    fn the_registry_stops_at_a_ceiling() {
        let registry = PeerRegistry::new();
        let ip = IpAddr::from([192, 168, 1, 5]);
        registry.upsert_manual(info("typed", None), ip, 53317);
        for i in 0..MAX_PEERS * 2 {
            registry.upsert(info(&format!("flood{i}"), Some(53317)), ip);
        }

        let peers = registry.snapshot();
        assert!(peers.len() <= MAX_PEERS, "{}", peers.len());
        assert!(
            peers.iter().any(|p| p.info.fingerprint == "typed"),
            "the manual peer was evicted"
        );
    }

    #[test]
    fn ipv6_base_urls_are_bracketed() {
        let registry = PeerRegistry::new();
        let ip: IpAddr = "fe80::1".parse().unwrap();
        registry.upsert_manual(info("typed", None), ip, 53317);
        let peers = registry.snapshot();
        assert_eq!(peers[0].base_url(), "http://[fe80::1]:53317");
    }
}
