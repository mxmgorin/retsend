//! Loopback send tests: the outbound worker drives our own receive server
//! over real TCP on 127.0.0.1 — client and server verified together, headless.

use retsend::net::discovery::PeerRegistry;
use retsend::net::protocol::{self, DeviceInfo};
use retsend::net::{server, NetShared, TransferSettings, Wake, WakeReason};
use retsend::transfer::outbound::{self, OutboundPhase};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct NoopWake;

impl Wake for NoopWake {
    fn wake(&self, _reason: WakeReason) {}
}

fn device(alias: &str) -> DeviceInfo {
    DeviceInfo {
        alias: alias.into(),
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

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("lsretro-send-{tag}-{}", protocol::random_token(4)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn start_receiver(auto_accept: bool) -> (Arc<NetShared>, String, PathBuf, impl FnOnce()) {
    let save_dir = temp_dir("recv");
    let shared = Arc::new(NetShared {
        me: Mutex::new(device("Receiver")),
        peers: PeerRegistry::new(),
        transfer: Mutex::new(TransferSettings {
            save_dir: save_dir.clone(),
            auto_accept,
        }),
        pending: Mutex::new(None),
        active: Mutex::new(None),
        outbound_active: AtomicBool::new(false),
        wake: Arc::new(NoopWake),
        shutdown: AtomicBool::new(false),
    });
    let (handle, port) = server::spawn(shared.clone(), 0, None).expect("server spawns");
    shared.me.lock().unwrap().port = Some(port);
    let base = format!("http://127.0.0.1:{port}");
    let stop = {
        let shared = shared.clone();
        let save_dir = save_dir.clone();
        move || {
            shared.shutdown.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", port));
            let _ = handle.join();
            let _ = std::fs::remove_dir_all(&save_dir);
        }
    };
    (shared, base, save_dir, stop)
}

fn wait_finished(session: &outbound::OutboundSession) -> OutboundPhase {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !session.is_finished() {
        assert!(Instant::now() < deadline, "send did not finish in time");
        std::thread::sleep(Duration::from_millis(10));
    }
    session.phase()
}

#[test]
fn sends_files_end_to_end() {
    let (_shared, base, save_dir, stop) = start_receiver(true);

    let src = temp_dir("src");
    std::fs::write(src.join("game.gbc"), b"ROM BYTES").unwrap();
    std::fs::write(src.join("save.dat"), b"SAVE").unwrap();

    let session = outbound::spawn(
        "Receiver".into(),
        base,
        device("Sender"),
        vec![src.join("game.gbc"), src.join("save.dat")],
        Arc::new(NoopWake),
    )
    .unwrap();

    assert_eq!(wait_finished(&session), OutboundPhase::Done);
    assert_eq!(session.done_count(), 2);
    assert_eq!(
        session.sent_total.load(Ordering::SeqCst),
        session.total_bytes
    );
    assert_eq!(
        std::fs::read(save_dir.join("game.gbc")).unwrap(),
        b"ROM BYTES"
    );
    assert_eq!(std::fs::read(save_dir.join("save.dat")).unwrap(), b"SAVE");

    std::fs::remove_dir_all(&src).unwrap();
    stop();
}

#[test]
fn decline_ends_the_send_as_declined() {
    let (shared, base, _save_dir, stop) = start_receiver(false);

    let src = temp_dir("src");
    std::fs::write(src.join("rom.gbc"), b"data").unwrap();

    let session = outbound::spawn(
        "Receiver".into(),
        base,
        device("Sender"),
        vec![src.join("rom.gbc")],
        Arc::new(NoopWake),
    )
    .unwrap();

    // The receiver's user declines the parked request.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(pending) = shared.pending.lock().unwrap().take() {
            pending.decline();
            break;
        }
        assert!(Instant::now() < deadline, "prepare never arrived");
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(wait_finished(&session), OutboundPhase::Declined);
    assert_eq!(session.done_count(), 0);

    std::fs::remove_dir_all(&src).unwrap();
    stop();
}

#[test]
fn sends_to_an_https_receiver() {
    use retsend::net::tls;

    tls::install_provider();
    let save_dir = temp_dir("recv-tls");
    let identity = tls::load_or_create(&save_dir).unwrap();
    let shared = Arc::new(NetShared {
        me: Mutex::new(DeviceInfo {
            fingerprint: identity.fingerprint.clone(),
            protocol: Some("https".into()),
            ..device("Receiver")
        }),
        peers: PeerRegistry::new(),
        transfer: Mutex::new(TransferSettings {
            save_dir: save_dir.clone(),
            auto_accept: true,
        }),
        pending: Mutex::new(None),
        active: Mutex::new(None),
        outbound_active: AtomicBool::new(false),
        wake: Arc::new(NoopWake),
        shutdown: AtomicBool::new(false),
    });
    let (handle, port) =
        server::spawn(shared.clone(), 0, Some(identity.server_config)).expect("server spawns");
    shared.me.lock().unwrap().port = Some(port);

    let src = temp_dir("src-tls");
    std::fs::write(src.join("rom.gbc"), b"ENCRYPTED BYTES").unwrap();

    let session = outbound::spawn(
        "Receiver".into(),
        format!("https://127.0.0.1:{port}"),
        device("Sender"),
        vec![src.join("rom.gbc")],
        Arc::new(NoopWake),
    )
    .unwrap();

    assert_eq!(wait_finished(&session), OutboundPhase::Done);
    assert_eq!(
        std::fs::read(save_dir.join("rom.gbc")).unwrap(),
        b"ENCRYPTED BYTES"
    );

    shared.shutdown.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect(("127.0.0.1", port));
    let _ = handle.join();
    std::fs::remove_dir_all(&src).unwrap();
    let _ = std::fs::remove_dir_all(&save_dir);
}

#[test]
fn missing_source_file_fails_to_spawn() {
    let (_shared, base, _save_dir, stop) = start_receiver(true);
    let result = outbound::spawn(
        "Receiver".into(),
        base,
        device("Sender"),
        vec![PathBuf::from("/nonexistent/definitely-missing.bin")],
        Arc::new(NoopWake),
    );
    assert!(result.is_err());
    stop();
}
