//! Networking: LocalSend discovery + the HTTP server, all blocking threads.
//!
//! Nothing in this module (or its children) touches SDL. Threads wake the UI
//! through the [`Wake`] trait — the app installs a `UserEventSender`-backed
//! impl, while headless integration tests pass a no-op.

pub mod client;
pub mod discovery;
pub mod httpd;
pub mod protocol;
pub mod server;
pub mod tls;

use crate::transfer::inbound::InboundSession;
use discovery::PeerRegistry;
use protocol::DeviceInfo;
use server::PendingRequest;
use std::net::{IpAddr, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub trait Wake: Send + Sync + 'static {
    fn wake(&self, reason: WakeReason);
}

#[derive(Copy, Clone, Debug)]
pub enum WakeReason {
    Peers,
    Incoming,
    Progress,
    Done,
}

/// Receive behavior the server consults per request. A snapshot of
/// [`crate::config::TransferConfig`]; the settings screen updates it live.
#[derive(Clone)]
pub struct TransferSettings {
    pub save_dir: PathBuf,
    pub auto_accept: bool,
}

impl From<&crate::config::TransferConfig> for TransferSettings {
    fn from(cfg: &crate::config::TransferConfig) -> Self {
        Self {
            save_dir: PathBuf::from(&cfg.save_dir),
            auto_accept: cfg.auto_accept,
        }
    }
}

/// State shared between the net threads and the UI thread.
pub struct NetShared {
    /// Our announced identity. `port` holds the *actually bound* TCP port.
    /// Mutex (not read-only) because settings will edit the alias live.
    pub me: Mutex<DeviceInfo>,
    pub peers: PeerRegistry,
    pub transfer: Mutex<TransferSettings>,
    /// A prepare-upload parked awaiting the user's accept/decline. At most
    /// one; its handler thread blocks with the HTTP response unsent.
    pub pending: Mutex<Option<PendingRequest>>,
    /// The accepted transfer currently receiving files. At most one.
    pub active: Mutex<Option<Arc<InboundSession>>>,
    /// An outbound send is in flight (set by the app around the worker's
    /// lifetime). Incoming prepare-uploads answer 409 while it's up — one
    /// transfer at a time keeps the UI honest on a small screen.
    pub outbound_active: AtomicBool,
    pub wake: Arc<dyn Wake>,
    /// Set by [`NetService::stop`]; every loop polls it and exits.
    pub shutdown: AtomicBool,
}

/// Handle owning the net threads: HTTP accept loop, multicast listener,
/// announcer. Constructed at startup; stopped and rebuilt when settings
/// change the port or alias.
pub struct NetService {
    pub shared: Arc<NetShared>,
    announcer_tx: mpsc::Sender<discovery::AnnouncerMsg>,
    handles: Vec<JoinHandle<()>>,
}

impl NetService {
    pub fn spawn(
        device: &crate::config::DeviceConfig,
        network: &crate::config::NetworkConfig,
        transfer: &crate::config::TransferConfig,
        data_dir: &std::path::Path,
        wake: Arc<dyn Wake>,
    ) -> std::io::Result<Self> {
        // HTTPS mode: the fingerprint is the SHA-256 of our persisted
        // certificate (peers remember devices by it). HTTP mode: just a
        // random self-ignore string, fresh per run.
        let identity = if network.https {
            Some(tls::load_or_create(data_dir).map_err(std::io::Error::other)?)
        } else {
            None
        };
        let me = DeviceInfo {
            alias: device.alias.clone(),
            version: protocol::PROTOCOL_VERSION.to_string(),
            device_model: Some(device.device_model.clone()),
            device_type: Some(device.device_type.clone()),
            fingerprint: identity
                .as_ref()
                .map(|i| i.fingerprint.clone())
                .unwrap_or_else(|| protocol::random_token(32)),
            port: None, // filled in below once the TCP listener binds
            protocol: Some(if identity.is_some() { "https" } else { "http" }.to_string()),
            download: Some(false),
            announce: None,
        };

        let shared = Arc::new(NetShared {
            me: Mutex::new(me),
            peers: PeerRegistry::new(),
            transfer: Mutex::new(TransferSettings::from(transfer)),
            pending: Mutex::new(None),
            active: Mutex::new(None),
            outbound_active: AtomicBool::new(false),
            wake,
            shutdown: AtomicBool::new(false),
        });

        // Bind TCP first: the announce must carry the real port.
        let tls = identity.map(|i| i.server_config);
        let (server_handle, actual_port) = server::spawn(shared.clone(), network.port, tls)?;
        shared.me.lock().unwrap().port = Some(actual_port);
        if actual_port != network.port {
            log::warn!(
                "port {} busy; listening on {actual_port} instead",
                network.port
            );
        }

        let listener_handle = discovery::spawn_listener(shared.clone())?;
        let (announcer_handle, announcer_tx) = discovery::spawn_announcer(
            shared.clone(),
            Duration::from_secs(network.announce_interval_secs),
        )?;

        Ok(Self {
            shared,
            announcer_tx,
            handles: vec![server_handle, listener_handle, announcer_handle],
        })
    }

    /// The TCP port actually bound (may differ from the configured one).
    pub fn http_port(&self) -> u16 {
        self.shared.me.lock().unwrap().port.unwrap_or(0)
    }

    /// Ask the announcer to multicast right now (the radar's manual refresh).
    pub fn re_announce(&self) {
        let _ = self.announcer_tx.send(discovery::AnnouncerMsg::ReAnnounce);
    }

    /// Stop and join all net threads (idempotent — the settings screen
    /// restarts the stack by stopping and reassigning). The announcer is
    /// poked via its channel, the UDP listener notices within its 1 s read
    /// timeout, and the accept loop is unblocked by a throwaway connection.
    pub fn stop(&mut self) {
        self.shared.shutdown.store(true, Ordering::SeqCst);
        let _ = self.announcer_tx.send(discovery::AnnouncerMsg::Shutdown);
        let port = self.http_port();
        let _ = TcpStream::connect(("127.0.0.1", port));
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

/// Our LAN address, for the header line of the Home screen. The
/// connected-UDP trick: no packet is sent, the OS just picks the route.
pub fn local_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip())
}
