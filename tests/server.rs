//! Headless round-trips against the real HTTP server on an ephemeral port —
//! no SDL involved, which is exactly what the `Wake` trait buys us.

use localsend_retro::net::discovery::PeerRegistry;
use localsend_retro::net::protocol::{self, DeviceInfo};
use localsend_retro::net::{server, NetShared, Wake, WakeReason};
use std::io::Read;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

struct NoopWake;

impl Wake for NoopWake {
    fn wake(&self, _reason: WakeReason) {}
}

fn test_me() -> DeviceInfo {
    DeviceInfo {
        alias: "Test Retro".into(),
        version: protocol::PROTOCOL_VERSION.into(),
        device_model: Some("CI".into()),
        device_type: Some("headless".into()),
        fingerprint: protocol::random_token(32),
        port: None,
        protocol: Some("http".into()),
        download: Some(false),
        announce: None,
    }
}

/// Server on an OS-assigned port; returns the shared state, the port, and a
/// closure joining the accept loop.
fn start_server() -> (Arc<NetShared>, u16, impl FnOnce()) {
    let shared = Arc::new(NetShared {
        me: Mutex::new(test_me()),
        peers: PeerRegistry::new(),
        wake: Arc::new(NoopWake),
        shutdown: AtomicBool::new(false),
    });
    let (handle, port) = server::spawn(shared.clone(), 0).expect("server spawns");
    shared.me.lock().unwrap().port = Some(port);
    let stop = {
        let shared = shared.clone();
        move || {
            shared.shutdown.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", port));
            let _ = handle.join();
        }
    };
    (shared, port, stop)
}

fn get(port: u16, path: &str) -> (u16, String) {
    let mut resp = match ureq::get(format!("http://127.0.0.1:{port}{path}")).call() {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(code)) => return (code, String::new()),
        Err(e) => panic!("request failed: {e}"),
    };
    let mut body = String::new();
    resp.body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .unwrap();
    (resp.status().as_u16(), body)
}

fn post(port: u16, path: &str, body: &str) -> (u16, String) {
    let req = ureq::post(format!("http://127.0.0.1:{port}{path}")).content_type("application/json");
    let mut resp = match req.send(body) {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(code)) => return (code, String::new()),
        Err(e) => panic!("request failed: {e}"),
    };
    let mut text = String::new();
    resp.body_mut()
        .as_reader()
        .read_to_string(&mut text)
        .unwrap();
    (resp.status().as_u16(), text)
}

#[test]
fn info_returns_our_device() {
    let (shared, port, stop) = start_server();
    let (status, body) = get(port, "/api/localsend/v2/info");
    assert_eq!(status, 200);
    let info: DeviceInfo = serde_json::from_str(&body).unwrap();
    assert_eq!(info.alias, "Test Retro");
    assert_eq!(info.port, Some(port));
    // `announce` is multicast-only and must not leak into REST responses.
    assert_eq!(info.announce, None);
    assert_eq!(info.fingerprint, shared.me.lock().unwrap().fingerprint);
    stop();
}

#[test]
fn register_exchanges_device_info() {
    let (shared, port, stop) = start_server();

    let peer = DeviceInfo {
        alias: "Phone".into(),
        fingerprint: "peer-fingerprint".into(),
        port: Some(53317),
        ..test_me()
    };
    let (status, body) = post(
        port,
        "/api/localsend/v2/register",
        &serde_json::to_string(&peer).unwrap(),
    );
    assert_eq!(status, 200);
    let ours: DeviceInfo = serde_json::from_str(&body).unwrap();
    assert_eq!(ours.alias, "Test Retro");

    let peers = shared.peers.snapshot();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].info.alias, "Phone");
    assert_eq!(peers[0].port, 53317);
    stop();
}

#[test]
fn register_rejects_garbage() {
    let (shared, port, stop) = start_server();
    let (status, _) = post(port, "/api/localsend/v2/register", "not json");
    assert_eq!(status, 400);
    assert!(shared.peers.snapshot().is_empty());
    stop();
}

#[test]
fn own_fingerprint_is_not_registered() {
    let (shared, port, stop) = start_server();
    let me = shared.me.lock().unwrap().clone();
    let (status, _) = post(
        port,
        "/api/localsend/v2/register",
        &serde_json::to_string(&me).unwrap(),
    );
    assert_eq!(status, 200);
    assert!(shared.peers.snapshot().is_empty());
    stop();
}

#[test]
fn unknown_routes_get_404_and_transfer_stubs_501() {
    let (_shared, port, stop) = start_server();
    assert_eq!(get(port, "/nope").0, 404);
    assert_eq!(get(port, "/api/localsend/v2/does-not-exist").0, 404);
    // M2 endpoints: alive but not implemented yet.
    assert_eq!(post(port, "/api/localsend/v2/prepare-upload", "{}").0, 501);
    stop();
}
