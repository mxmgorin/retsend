//! Headless round-trips against the real HTTP server on an ephemeral port —
//! no SDL involved, which is exactly what the `Wake` trait buys us.

use retsend::net::discovery::PeerRegistry;
use retsend::net::protocol::{self, DeviceInfo};
use retsend::net::{manual, server, NetShared, TransferSettings, Wake, WakeReason};
use rustls::pki_types::CertificateDer;
use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
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

/// A fresh scratch dir under the OS temp dir; removed by `stop`.
fn temp_save_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lsretro-recv-{}", protocol::random_token(4)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Server on an OS-assigned port; returns the shared state, the port, and a
/// closure joining the accept loop and sweeping the save dir.
fn start_server(auto_accept: bool) -> (Arc<NetShared>, u16, impl FnOnce()) {
    let save_dir = temp_save_dir();
    let shared = Arc::new(NetShared {
        me: Mutex::new(test_me()),
        peers: PeerRegistry::new(),
        transfer: Mutex::new(TransferSettings {
            save_dir: save_dir.clone(),
            auto_accept,
            overwrite: true,
            auto_routes: false,
            keep_folders: true,
            routes: Default::default(),
        }),
        pending: Mutex::new(None),
        active: Mutex::new(None),
        outbound_active: AtomicBool::new(false),
        notices: Mutex::new(Vec::new()),
        wake: Arc::new(NoopWake),
        shutdown: AtomicBool::new(false),
    });
    let (handle, port) = server::spawn(shared.clone(), 0, None).expect("server spawns");
    shared.me.lock().unwrap().port = Some(port);
    let stop = {
        let shared = shared.clone();
        move || {
            shared.shutdown.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", port));
            let _ = handle.join();
            let _ = std::fs::remove_dir_all(&save_dir);
        }
    };
    (shared, port, stop)
}

fn save_dir_of(shared: &NetShared) -> PathBuf {
    shared.transfer.lock().unwrap().save_dir.clone()
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
    let (shared, port, stop) = start_server(false);
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
    let (shared, port, stop) = start_server(false);

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
    let (shared, port, stop) = start_server(false);
    let (status, _) = post(port, "/api/localsend/v2/register", "not json");
    assert_eq!(status, 400);
    assert!(shared.peers.snapshot().is_empty());
    stop();
}

#[test]
fn own_fingerprint_is_not_registered() {
    let (shared, port, stop) = start_server(false);
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
fn unknown_routes_get_404_and_reverse_mode_501() {
    let (_shared, port, stop) = start_server(false);
    assert_eq!(get(port, "/nope").0, 404);
    assert_eq!(get(port, "/api/localsend/v2/does-not-exist").0, 404);
    // Reverse mode (browser download) is out of scope for v1.
    assert_eq!(post(port, "/api/localsend/v2/prepare-download", "").0, 501);
    // Malformed prepare bodies are rejected, not parked.
    assert_eq!(post(port, "/api/localsend/v2/prepare-upload", "{}").0, 400);
    stop();
}

// ---- receive flow ----------------------------------------------------------

fn prepare_body(files: &[(&str, &str, u64)]) -> String {
    let files: serde_json::Map<String, serde_json::Value> = files
        .iter()
        .map(|(id, name, size)| {
            (
                id.to_string(),
                serde_json::json!({
                    "id": id, "fileName": name, "size": size,
                    "fileType": "application/octet-stream",
                }),
            )
        })
        .collect();
    serde_json::json!({
        "info": { "alias": "Phone", "version": "2.1", "fingerprint": "phone-fp" },
        "files": files,
    })
    .to_string()
}

fn upload(port: u16, session_id: &str, file_id: &str, token: &str, bytes: &[u8]) -> u16 {
    let url = format!(
        "http://127.0.0.1:{port}/api/localsend/v2/upload?sessionId={session_id}&fileId={file_id}&token={token}"
    );
    match ureq::post(url)
        .content_type("application/octet-stream")
        .send(bytes)
    {
        Ok(r) => r.status().as_u16(),
        Err(ureq::Error::StatusCode(code)) => code,
        Err(e) => panic!("upload failed: {e}"),
    }
}

fn tokens_of(body: &str) -> (String, std::collections::BTreeMap<String, String>) {
    let resp: protocol::PrepareUploadResponse = serde_json::from_str(body).unwrap();
    (resp.session_id, resp.files)
}

/// Poll until `f` yields, or panic after 5 s — for handshake tests where the
/// prepare request parks on a background thread.
fn wait_for<T>(mut f: impl FnMut() -> Option<T>) -> T {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(v) = f() {
            return v;
        }
        assert!(std::time::Instant::now() < deadline, "timed out waiting");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn auto_accept_receives_files() {
    let (shared, port, stop) = start_server(true);

    let (status, body) = post(
        port,
        "/api/localsend/v2/prepare-upload",
        &prepare_body(&[("a", "game.gbc", 5), ("b", "save.dat", 3)]),
    );
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);

    assert_eq!(upload(port, &session_id, "a", &tokens["a"], b"hello"), 200);
    assert_eq!(upload(port, &session_id, "b", &tokens["b"], b"sav"), 200);

    let dir = save_dir_of(&shared);
    assert_eq!(std::fs::read(dir.join("game.gbc")).unwrap(), b"hello");
    assert_eq!(std::fs::read(dir.join("save.dat")).unwrap(), b"sav");
    // The finished session no longer blocks the next transfer.
    assert!(shared.active.lock().unwrap().is_none());
    stop();
}

/// The `overwrite` setting decides what a second transfer of the same name
/// does, and it is read per request — the settings screen edits it live.
#[test]
fn overwrite_setting_decides_whether_a_resend_replaces_the_file() {
    let (shared, port, stop) = start_server(true);
    let dir = save_dir_of(&shared);

    let send = |body: &[u8]| {
        let (status, response) = post(
            port,
            "/api/localsend/v2/prepare-upload",
            &prepare_body(&[("a", "game.gbc", body.len() as u64)]),
        );
        assert_eq!(status, 200);
        let (session_id, tokens) = tokens_of(&response);
        assert_eq!(upload(port, &session_id, "a", &tokens["a"], body), 200);
    };

    send(b"first");
    send(b"second");
    assert_eq!(std::fs::read(dir.join("game.gbc")).unwrap(), b"second");
    assert!(!dir.join("game (1).gbc").exists());

    shared.transfer.lock().unwrap().overwrite = false;
    send(b"third");
    assert_eq!(std::fs::read(dir.join("game.gbc")).unwrap(), b"second");
    assert_eq!(std::fs::read(dir.join("game (1).gbc")).unwrap(), b"third");

    stop();
}

#[test]
fn manual_accept_parks_until_the_user_agrees() {
    let (shared, port, stop) = start_server(false);

    let handle = std::thread::spawn(move || {
        post(
            port,
            "/api/localsend/v2/prepare-upload",
            &prepare_body(&[("a", "rom.gbc", 4)]),
        )
    });

    // The handler is parked with the response unsent; the modal's data is up.
    let (sender_alias, total) = wait_for(|| {
        let pending = shared.pending.lock().unwrap();
        pending
            .as_ref()
            .map(|p| (p.sender.alias.clone(), p.total_bytes))
    });
    assert_eq!(sender_alias, "Phone");
    assert_eq!(total, 4);

    // A second prepare while one is pending is blocked.
    assert_eq!(
        post(
            port,
            "/api/localsend/v2/prepare-upload",
            &prepare_body(&[("x", "other.bin", 1)]),
        )
        .0,
        409
    );

    // The user presses A.
    let pending = shared.pending.lock().unwrap().take().unwrap();
    pending.accept(save_dir_of(&shared));

    let (status, body) = handle.join().unwrap();
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);
    assert_eq!(upload(port, &session_id, "a", &tokens["a"], b"data"), 200);
    assert_eq!(
        std::fs::read(save_dir_of(&shared).join("rom.gbc")).unwrap(),
        b"data"
    );
    stop();
}

/// The modal promises destinations with the save routes already applied: a
/// routed extension shows its folder, and a request the routes split up names
/// every folder it will use.
#[test]
fn the_parked_request_names_the_routed_destinations() {
    let (shared, port, stop) = start_server(false);
    let save_dir = save_dir_of(&shared);
    shared.transfer.lock().unwrap().routes = [("gbc".to_string(), "gb".to_string())].into();

    let posted = std::thread::spawn(move || {
        post(
            port,
            "/api/localsend/v2/prepare-upload",
            // Name-sorted, so the routed `.gbc` leads and `notes.txt` falls
            // back to the save dir itself.
            &prepare_body(&[("a", "Zelda.gbc", 4), ("b", "notes.txt", 1)]),
        )
    });

    let dests = wait_for(|| {
        shared
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| p.dests.clone())
    });
    assert_eq!(
        dests,
        [
            save_dir.join("gb").display().to_string(),
            save_dir.display().to_string(),
        ]
    );

    shared.pending.lock().unwrap().take().unwrap().decline();
    assert_eq!(posted.join().unwrap().0, 403);
    stop();
}

#[test]
fn decline_answers_403() {
    let (shared, port, stop) = start_server(false);

    let handle = std::thread::spawn(move || {
        post(
            port,
            "/api/localsend/v2/prepare-upload",
            &prepare_body(&[("a", "rom.gbc", 4)]),
        )
    });

    let pending = wait_for(|| shared.pending.lock().unwrap().take());
    pending.decline();

    assert_eq!(handle.join().unwrap().0, 403);
    assert!(shared.active.lock().unwrap().is_none());
    stop();
}

#[test]
fn upload_rejects_bad_credentials() {
    let (shared, port, stop) = start_server(true);
    let (status, body) = post(
        port,
        "/api/localsend/v2/prepare-upload",
        &prepare_body(&[("a", "x.bin", 1)]),
    );
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);

    assert_eq!(upload(port, "bogus-session", "a", &tokens["a"], b"x"), 403);
    assert_eq!(upload(port, &session_id, "a", "bogus-token", b"x"), 403);
    assert_eq!(upload(port, &session_id, "nope", &tokens["a"], b"x"), 403);
    // Missing query params.
    assert_eq!(post(port, "/api/localsend/v2/upload", "x").0, 400);

    assert!(!save_dir_of(&shared).join("x.bin").exists());
    stop();
}

#[test]
fn sender_cancel_clears_the_session() {
    let (shared, port, stop) = start_server(true);
    let (status, body) = post(
        port,
        "/api/localsend/v2/prepare-upload",
        &prepare_body(&[("a", "x.bin", 1)]),
    );
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);

    let (status, _) = post(
        port,
        &format!("/api/localsend/v2/cancel?sessionId={session_id}"),
        "",
    );
    assert_eq!(status, 200);
    assert!(shared.active.lock().unwrap().is_none());
    // Uploads for the cancelled session are refused.
    assert_eq!(upload(port, &session_id, "a", &tokens["a"], b"x"), 403);
    // Cancel is idempotent.
    assert_eq!(
        post(
            port,
            &format!("/api/localsend/v2/cancel?sessionId={session_id}"),
            "",
        )
        .0,
        200
    );
    stop();
}

#[test]
fn hostile_file_names_stay_inside_save_dir() {
    let (shared, port, stop) = start_server(true);
    let (status, body) = post(
        port,
        "/api/localsend/v2/prepare-upload",
        &prepare_body(&[("a", "../../evil.sh", 4)]),
    );
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);
    assert_eq!(upload(port, &session_id, "a", &tokens["a"], b"boom"), 200);

    let dir = save_dir_of(&shared);
    assert_eq!(std::fs::read(dir.join("evil.sh")).unwrap(), b"boom");
    assert!(!dir.parent().unwrap().join("evil.sh").exists());
    stop();
}

/// A folder transfer as the official app sends it: `fileName` carries the
/// relative path, which we rebuild under the save dir instead of routing by
/// extension.
#[test]
fn a_folder_transfer_lands_as_a_folder() {
    let (shared, port, stop) = start_server(true);
    shared.transfer.lock().unwrap().routes = [("gbc".to_string(), "gb".to_string())].into();

    let (status, body) = post(
        port,
        "/api/localsend/v2/prepare-upload",
        &prepare_body(&[
            ("a", "Zelda/Zelda.gbc", 4),
            ("b", "Zelda/saves/Zelda.sav", 4),
            ("c", "loose.gbc", 4),
        ]),
    );
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);
    for id in ["a", "b", "c"] {
        assert_eq!(upload(port, &session_id, id, &tokens[id], b"data"), 200);
    }

    let dir = save_dir_of(&shared);
    assert!(dir.join("Zelda/Zelda.gbc").is_file());
    assert!(dir.join("Zelda/saves/Zelda.sav").is_file());
    // The route still places a file the sender sent loose.
    assert!(dir.join("gb/loose.gbc").is_file());
    stop();
}

#[test]
fn https_serves_info_with_certificate_fingerprint() {
    use retsend::net::{client, tls};

    tls::install_provider();
    let dir = temp_save_dir();
    let identity = tls::load_or_create(&dir).unwrap();
    let fingerprint = identity.fingerprint.clone();
    // Installed against a server built `with_no_client_auth`: a peer that
    // never asks must see the same handshake it always did.
    client::set_client_cert(identity.client_cert.clone());

    let shared = Arc::new(NetShared {
        me: Mutex::new(DeviceInfo {
            fingerprint: fingerprint.clone(),
            protocol: Some("https".into()),
            ..test_me()
        }),
        peers: PeerRegistry::new(),
        transfer: Mutex::new(TransferSettings {
            save_dir: dir.clone(),
            auto_accept: true,
            overwrite: true,
            auto_routes: false,
            keep_folders: true,
            routes: Default::default(),
        }),
        pending: Mutex::new(None),
        active: Mutex::new(None),
        outbound_active: AtomicBool::new(false),
        notices: Mutex::new(Vec::new()),
        wake: Arc::new(NoopWake),
        shutdown: AtomicBool::new(false),
    });
    let (handle, port) =
        server::spawn(shared.clone(), 0, Some(identity.server_config)).expect("server spawns");
    shared.me.lock().unwrap().port = Some(port);

    // The peer-facing agent skips CA verification (trust = fingerprint).
    let mut resp = client::agent(None)
        .get(format!("https://127.0.0.1:{port}/api/localsend/v2/info"))
        .call()
        .expect("https request succeeds");
    let mut body = String::new();
    resp.body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .unwrap();
    let info: DeviceInfo = serde_json::from_str(&body).unwrap();
    assert_eq!(info.fingerprint, fingerprint);
    assert_eq!(info.protocol.as_deref(), Some("https"));

    // A default (verifying) client must reject the self-signed certificate.
    assert!(
        ureq::get(format!("https://127.0.0.1:{port}/api/localsend/v2/info"))
            .call()
            .is_err()
    );

    shared.shutdown.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect(("127.0.0.1", port));
    let _ = handle.join();
    let _ = std::fs::remove_dir_all(&dir);
}

/// A LocalSend receiver that isn't also serving its web UI makes client auth
/// mandatory (`CustomClientCertVerifier` in their core). This stands in for
/// one: it demands a certificate without caring which, so a client presenting
/// none is dropped with `CertificateRequired`.
#[derive(Debug)]
struct DemandsClientCert;

impl rustls::server::danger::ClientCertVerifier for DemandsClientCert {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        Ok(rustls::server::danger::ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &verify_schemes())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &verify_schemes())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        verify_schemes().supported_schemes()
    }
}

fn verify_schemes() -> rustls::crypto::WebPkiSupportedAlgorithms {
    rustls::crypto::CryptoProvider::get_default()
        .expect("net::tls::install_provider ran before the server was built")
        .signature_verification_algorithms
}

#[test]
fn a_peer_demanding_a_client_certificate_gets_ours() {
    use retsend::net::{client, tls};
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

    tls::install_provider();
    let dir = temp_save_dir();
    let identity = tls::load_or_create(&dir).unwrap();
    client::set_client_cert(identity.client_cert.clone());

    // The peer's own self-signed identity; our client trusts fingerprints,
    // not chains, so any certificate serves.
    let peer = rcgen::generate_simple_self_signed(vec!["peer".to_string()]).unwrap();
    let server_config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(Arc::new(DemandsClientCert))
        .with_single_cert(
            vec![CertificateDer::from(peer.cert.der().to_vec())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(peer.signing_key.serialize_der())),
        )
        .expect("peer server config");

    let shared = Arc::new(NetShared {
        me: Mutex::new(DeviceInfo {
            protocol: Some("https".into()),
            ..test_me()
        }),
        peers: PeerRegistry::new(),
        transfer: Mutex::new(TransferSettings {
            save_dir: dir.clone(),
            auto_accept: true,
            overwrite: true,
            auto_routes: false,
            keep_folders: true,
            routes: Default::default(),
        }),
        pending: Mutex::new(None),
        active: Mutex::new(None),
        outbound_active: AtomicBool::new(false),
        notices: Mutex::new(Vec::new()),
        wake: Arc::new(NoopWake),
        shutdown: AtomicBool::new(false),
    });
    let (handle, port) =
        server::spawn(shared.clone(), 0, Some(Arc::new(server_config))).expect("server spawns");
    shared.me.lock().unwrap().port = Some(port);

    let url = format!("https://127.0.0.1:{port}/api/localsend/v2/info");
    assert!(
        client::agent(None).get(&url).call().is_ok(),
        "the peer-facing agent must present our certificate"
    );

    // Without one the handshake dies — the failure users saw as
    // "Send failed: certificate required".
    let bare: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .disable_verification(true)
                .build(),
        )
        .build()
        .into();
    assert!(bare.get(&url).call().is_err());

    shared.shutdown.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect(("127.0.0.1", port));
    let _ = handle.join();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn incoming_prepare_is_blocked_while_sending() {
    let (shared, port, stop) = start_server(true);
    shared.outbound_active.store(true, Ordering::SeqCst);
    assert_eq!(
        post(
            port,
            "/api/localsend/v2/prepare-upload",
            &prepare_body(&[("a", "x.bin", 1)]),
        )
        .0,
        409
    );
    shared.outbound_active.store(false, Ordering::SeqCst);
    assert_eq!(
        post(
            port,
            "/api/localsend/v2/prepare-upload",
            &prepare_body(&[("a", "x.bin", 1)]),
        )
        .0,
        200
    );
    stop();
}

#[test]
fn size_mismatch_fails_the_file_with_500() {
    let (shared, port, stop) = start_server(true);
    let (status, body) = post(
        port,
        "/api/localsend/v2/prepare-upload",
        &prepare_body(&[("a", "big.bin", 100)]),
    );
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);
    assert_eq!(upload(port, &session_id, "a", &tokens["a"], b"short"), 500);

    let dir = save_dir_of(&shared);
    assert!(!dir.join("big.bin").exists());
    assert!(!dir.join("big.bin.part").exists());
    stop();
}

/// The other side of the manual add: no server of our own is needed to probe
/// one, only an identity to register with.
fn prober() -> Arc<NetShared> {
    Arc::new(NetShared {
        me: Mutex::new(DeviceInfo {
            alias: "Prober".into(),
            // A port the peer can dial us back on; the registry drops peers
            // that announce none.
            port: Some(53317),
            ..test_me()
        }),
        peers: PeerRegistry::new(),
        transfer: Mutex::new(TransferSettings {
            save_dir: std::env::temp_dir(),
            auto_accept: false,
            overwrite: true,
            auto_routes: false,
            keep_folders: true,
            routes: Default::default(),
        }),
        pending: Mutex::new(None),
        active: Mutex::new(None),
        outbound_active: AtomicBool::new(false),
        notices: Mutex::new(Vec::new()),
        wake: Arc::new(NoopWake),
        shutdown: AtomicBool::new(false),
    })
}

/// The probe's outcome, once its thread has reported one.
fn probe_notice(shared: &Arc<NetShared>) -> String {
    wait_for(|| shared.take_notices().pop())
}

#[test]
fn a_typed_address_registers_both_ways() {
    let (shared, port, stop) = start_server(false);
    let prober = prober();

    manual::spawn_probe(prober.clone(), ([127, 0, 0, 1], port).into());
    // The notice is queued last, so the peer is in place once it shows.
    assert_eq!(probe_notice(&prober), "Added Test Retro");

    let peers = prober.peers.snapshot();
    assert_eq!(peers.len(), 1);
    assert!(peers[0].manual, "typed peers are sticky");
    // The scheme that answered: https is tried first and this server is plain.
    assert_eq!(peers[0].base_url(), format!("http://127.0.0.1:{port}"));
    assert_eq!(
        peers[0].info.fingerprint,
        shared.me.lock().unwrap().fingerprint
    );
    // /register is symmetric — the peer learned about us as well.
    assert_eq!(shared.peers.snapshot().len(), 1);
    stop();
}

#[test]
fn a_dead_address_says_so() {
    let prober = prober();
    // Port 1 is privileged and unbound here: the connection is refused at once.
    manual::spawn_probe(prober.clone(), ([127, 0, 0, 1], 1).into());
    assert_eq!(probe_notice(&prober), "No device at 127.0.0.1:1");
    assert!(prober.peers.snapshot().is_empty());
}

#[test]
fn a_chosen_folder_takes_the_files_whatever_the_routes_say() {
    let (shared, port, stop) = start_server(false);
    shared.transfer.lock().unwrap().routes =
        std::collections::BTreeMap::from([("gbc".to_string(), "gb".to_string())]);
    let save_dir = save_dir_of(&shared);
    let chosen = save_dir.join("elsewhere");

    let handle = std::thread::spawn(move || {
        post(
            port,
            "/api/localsend/v2/prepare-upload",
            &prepare_body(&[("a", "rom.gbc", 4)]),
        )
    });

    // The modal offers the route's folder…
    let dests = wait_for(|| {
        shared
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| p.dests.clone())
    });
    assert_eq!(dests, vec![save_dir.join("gb").display().to_string()]);

    // …but the user browsed to another one, and that answer is final.
    let pending = shared.pending.lock().unwrap().take().expect("still parked");
    pending.accept_into(chosen.clone());

    let (status, body) = handle.join().unwrap();
    assert_eq!(status, 200);
    let (session_id, tokens) = tokens_of(&body);
    assert_eq!(upload(port, &session_id, "a", &tokens["a"], b"rom!"), 200);

    assert_eq!(std::fs::read(chosen.join("rom.gbc")).unwrap(), b"rom!");
    assert!(!save_dir.join("gb").exists(), "the route must not apply");
    stop();
}
