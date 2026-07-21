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

#[derive(Clone, Debug)]
pub struct Peer {
    pub info: DeviceInfo,
    pub ip: IpAddr,
    /// From the announce/register body; the port we dial for transfers.
    pub port: u16,
    pub last_seen: Instant,
}

impl Peer {
    /// URL scheme this peer serves ("http"/"https"), from its announce.
    pub fn scheme(&self) -> &str {
        self.info.protocol.as_deref().unwrap_or("http")
    }

    /// Base URL for the peer's REST endpoints.
    pub fn base_url(&self) -> String {
        format!("{}://{}:{}", self.scheme(), self.ip, self.port)
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
        let mut peers = self.peers.lock().unwrap();
        let existing = peers.get(&info.fingerprint);
        let changed = match existing {
            Some(p) => p.info.alias != info.alias || p.ip != ip || p.port != port,
            None => true,
        };
        peers.insert(
            info.fingerprint.clone(),
            Peer {
                info,
                ip,
                port,
                last_seen: Instant::now(),
            },
        );
        changed
    }

    /// Live peers sorted by alias; prunes expired entries as it goes.
    pub fn snapshot(&self) -> Vec<Peer> {
        let mut peers = self.peers.lock().unwrap();
        peers.retain(|_, p| p.last_seen.elapsed() < PEER_TTL);
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

    std::thread::Builder::new()
        .name("discovery".into())
        .spawn(move || {
            let mut buf = vec![0u8; MAX_PACKET];
            loop {
                if shared.shutdown.load(Ordering::SeqCst) {
                    return;
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
                if wants_reply {
                    reply_to_announce(&shared, info, src.ip());
                }
            }
        })
}

fn bind_multicast(port: u16) -> std::io::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
    let udp: UdpSocket = socket.into();
    // INADDR_ANY picks the default-route interface; handhelds have a single
    // wifi NIC, so that's always right there. `set_multicast_if_v4` is the
    // knob if multi-NIC desktops ever misbehave.
    udp.join_multicast_v4(&protocol::MULTICAST_GROUP, &Ipv4Addr::UNSPECIFIED)?;
    udp.set_multicast_loop_v4(true)?;
    udp.set_read_timeout(Some(Duration::from_secs(1)))?;
    Ok(udp)
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
