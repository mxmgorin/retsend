//! The HTTP accept loop, endpoint routing, and the receive flow's heart: the
//! prepare-upload handler that parks — HTTP response unsent — until the user
//! answers the incoming-request modal (or the 60 s deadline declines for
//! them). `/upload` streams into the accepted [`InboundSession`]; `/cancel`
//! tears it down. `prepare-download` (reverse mode) stays 501 by design.

use super::httpd;
use super::protocol::{self, DeviceInfo, PrepareUploadRequest, PrepareUploadResponse};
use super::{NetShared, WakeReason};
use crate::transfer::inbound::InboundSession;
use std::io::{BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// `/register` bodies are tiny; cap far above any real one.
const MAX_JSON_BODY: u64 = 64 * 1024;
/// `/prepare-upload` carries metadata for every file — allow big batches.
const MAX_PREPARE_BODY: u64 = 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a prepare-upload waits for the user before declining. The
/// official app's own dialog waits about this long.
pub const DECISION_TIMEOUT: Duration = Duration::from_secs(60);

/// The user's answer to an incoming request.
pub enum Decision {
    Accept { save_dir: PathBuf },
    Decline,
}

/// A parked `/prepare-upload` awaiting the user. Created by the handler
/// thread, surfaced to the UI via [`NetShared::pending`]. The consuming
/// methods make a double answer impossible; dropping it unanswered leaves the
/// handler to its timeout, which declines.
pub struct PendingRequest {
    pub sender: DeviceInfo,
    /// Name-sorted for display.
    pub files: Vec<protocol::FileMeta>,
    pub total_bytes: u64,
    /// The modal renders its countdown from this against [`DECISION_TIMEOUT`].
    pub received_at: Instant,
    decision_tx: mpsc::SyncSender<Decision>,
}

impl PendingRequest {
    /// Unparks the handler thread with a yes; it answers 200 with tokens and
    /// installs the session into [`NetShared::active`].
    pub fn accept(self, save_dir: PathBuf) {
        let _ = self.decision_tx.send(Decision::Accept { save_dir });
    }

    pub fn decline(self) {
        let _ = self.decision_tx.send(Decision::Decline);
    }
}

/// The transport under the HTTP parser: plain TCP or rustls (the protocol's
/// https mode). The handshake happens lazily on first read/write.
enum ServerStream {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ServerConnection, TcpStream>>),
}

impl Read for ServerStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ServerStream::Plain(s) => s.read(buf),
            ServerStream::Tls(s) => s.read(buf),
        }
    }
}

impl Write for ServerStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            ServerStream::Plain(s) => s.write(buf),
            ServerStream::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            ServerStream::Plain(s) => s.flush(),
            ServerStream::Tls(s) => s.flush(),
        }
    }
}

/// Bind the preferred port, falling back through the next 9 (the official app
/// may hold 53317 on a dev machine). Returns the accept-loop handle and the
/// port actually bound — the caller stores it into `me.port` before discovery
/// starts, so announces always carry a dialable port. `tls` switches every
/// connection to the protocol's https mode.
pub fn spawn(
    shared: Arc<NetShared>,
    preferred_port: u16,
    tls: Option<Arc<rustls::ServerConfig>>,
) -> std::io::Result<(JoinHandle<()>, u16)> {
    let mut last_err = None;
    let mut bound = None;
    for offset in 0..10u16 {
        let port = preferred_port.saturating_add(offset);
        match TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)) {
            Ok(listener) => {
                bound = Some(listener);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let listener = match bound {
        Some(l) => l,
        None => return Err(last_err.expect("no bind attempts made")),
    };
    // Re-read from the OS: asking for port 0 (tests) binds an ephemeral one.
    let port = listener.local_addr()?.port();
    log::info!("http server listening on 0.0.0.0:{port}");

    let handle = std::thread::Builder::new()
        .name("http-accept".into())
        .spawn(move || loop {
            let (stream, peer_addr) = match listener.accept() {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("accept failed: {e}");
                    continue;
                }
            };
            // `NetService::stop` sets the flag, then self-connects to unblock
            // this accept — that throwaway connection lands here and exits.
            if shared.shutdown.load(Ordering::SeqCst) {
                return;
            }
            let shared = shared.clone();
            let tls = tls.clone();
            let spawned = std::thread::Builder::new()
                .name(format!("http-{peer_addr}"))
                .spawn(move || handle_connection(stream, &shared, tls));
            if let Err(e) = spawned {
                log::warn!("could not spawn connection thread: {e}");
            }
        })?;

    Ok((handle, port))
}

fn handle_connection(
    stream: TcpStream,
    shared: &Arc<NetShared>,
    tls: Option<Arc<rustls::ServerConfig>>,
) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let peer_ip = match stream.peer_addr() {
        Ok(a) => a.ip(),
        Err(_) => return, // connection already gone
    };

    let stream = match tls {
        Some(config) => match rustls::ServerConnection::new(config) {
            Ok(conn) => ServerStream::Tls(Box::new(rustls::StreamOwned::new(conn, stream))),
            Err(e) => {
                log::warn!("tls setup for {peer_ip} failed: {e}");
                return;
            }
        },
        None => ServerStream::Plain(stream),
    };

    let mut reader = BufReader::new(stream);
    let request = match httpd::parse_request(&mut reader) {
        Ok(r) => r,
        Err(e) => {
            log::debug!("bad request from {peer_ip}: {} ({})", e.message, e.status);
            let _ = httpd::respond_empty(reader.get_mut(), e.status);
            return;
        }
    };
    log::debug!("{} {} from {peer_ip}", request.method, request.path);

    let route = request
        .path
        .strip_prefix(protocol::API_PREFIX)
        .map(|r| r.to_string());
    let result = match (request.method.as_str(), route.as_deref()) {
        ("GET", Some("/info")) => {
            let me = { shared.me.lock().unwrap().clone() };
            let json = serde_json::to_string(&public_info(me)).expect("DeviceInfo serializes");
            httpd::respond_json(reader.get_mut(), 200, &json)
        }
        ("POST", Some("/register")) => handle_register(&mut reader, &request, shared, peer_ip),
        ("POST", Some("/prepare-upload")) => handle_prepare_upload(&mut reader, &request, shared),
        ("POST", Some("/upload")) => handle_upload(&mut reader, &request, shared),
        ("POST", Some("/cancel")) => handle_cancel(reader.get_mut(), &request, shared),
        // Reverse mode (browser download) is out of scope for v1.
        ("POST" | "GET", Some("/prepare-download" | "/download")) => {
            httpd::respond_empty(reader.get_mut(), 501)
        }
        (_, Some(_) | None) => httpd::respond_empty(reader.get_mut(), 404),
    };
    if let Err(e) = result {
        log::debug!("response to {peer_ip} failed: {e}");
    }
}

/// The peer POSTs its device info; we upsert it and answer with ours. This is
/// the TCP half of discovery — it works even when multicast RX is broken on
/// the peer's (or our) wifi chip.
fn handle_register<S: Read + Write>(
    reader: &mut BufReader<S>,
    request: &httpd::Request,
    shared: &Arc<NetShared>,
    peer_ip: std::net::IpAddr,
) -> std::io::Result<()> {
    let Some(info) = read_json_body::<_, DeviceInfo>(reader, request, MAX_JSON_BODY)? else {
        return httpd::respond_empty(reader.get_mut(), 400);
    };

    let me = { shared.me.lock().unwrap().clone() };
    if info.fingerprint != me.fingerprint && shared.peers.upsert(info.clone(), peer_ip) {
        log::info!("registered `{}` at {peer_ip}", info.alias);
        shared.wake.wake(WakeReason::Peers);
    }

    let json = serde_json::to_string(&public_info(me)).expect("DeviceInfo serializes");
    httpd::respond_json(reader.get_mut(), 200, &json)
}

/// The receive handshake. Parses the sender's file list, surfaces it to the
/// UI as a [`PendingRequest`], and blocks on the decision channel with the
/// HTTP response unsent — `recv_timeout` is the whole trick. `auto_accept`
/// ("quick save") short-circuits the park entirely.
fn handle_prepare_upload<S: Read + Write>(
    reader: &mut BufReader<S>,
    request: &httpd::Request,
    shared: &Arc<NetShared>,
) -> std::io::Result<()> {
    let Some(prepare) =
        read_json_body::<_, PrepareUploadRequest>(reader, request, MAX_PREPARE_BODY)?
    else {
        return httpd::respond_empty(reader.get_mut(), 400);
    };
    let mut files: Vec<protocol::FileMeta> = prepare.files.into_values().collect();
    if files.is_empty() {
        return httpd::respond_empty(reader.get_mut(), 400);
    }
    files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    let total_bytes = files.iter().map(|f| f.size).sum();

    // One transfer at a time: while we're sending, don't also receive.
    if shared.outbound_active.load(Ordering::SeqCst) {
        return httpd::respond_empty(reader.get_mut(), 409);
    }

    // A finished or abandoned session must not block new transfers forever.
    {
        let mut active = shared.active.lock().unwrap();
        if let Some(session) = &*active {
            if session.is_finished() || session.is_stale() {
                *active = None;
            }
        }
        if active.is_some() {
            return httpd::respond_empty(reader.get_mut(), 409);
        }
    }

    let settings = shared.transfer.lock().unwrap().clone();
    if settings.auto_accept {
        return start_session(
            reader.get_mut(),
            shared,
            prepare.info,
            files,
            settings.save_dir,
        );
    }

    let (tx, rx) = mpsc::sync_channel::<Decision>(1);
    {
        let mut pending = shared.pending.lock().unwrap();
        if pending.is_some() {
            return httpd::respond_empty(reader.get_mut(), 409);
        }
        *pending = Some(PendingRequest {
            sender: prepare.info.clone(),
            files: files.clone(),
            total_bytes,
            received_at: Instant::now(),
            decision_tx: tx,
        });
    }
    shared.wake.wake(WakeReason::Incoming); // pops the modal on the UI thread

    // Parked here until the user presses A/B or the deadline passes. On
    // timeout the pending entry is still ours to clear; if the UI raced us
    // and just took it, its accept lands on a dropped receiver (harmless)
    // and it will notice no session ever appeared.
    let decision = rx.recv_timeout(DECISION_TIMEOUT);
    shared.pending.lock().unwrap().take();
    match decision {
        Ok(Decision::Accept { save_dir }) => {
            start_session(reader.get_mut(), shared, prepare.info, files, save_dir)
        }
        Ok(Decision::Decline) | Err(_) => httpd::respond_empty(reader.get_mut(), 403),
    }
}

/// Create the session, install it as active, answer 200 with the tokens.
fn start_session<S: Write>(
    stream: &mut S,
    shared: &Arc<NetShared>,
    sender: DeviceInfo,
    files: Vec<protocol::FileMeta>,
    save_dir: PathBuf,
) -> std::io::Result<()> {
    let session = match InboundSession::new(sender.alias.clone(), files, &save_dir) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::error!("could not start session in `{}`: {e}", save_dir.display());
            return httpd::respond_empty(stream, 500);
        }
    };
    {
        let mut active = shared.active.lock().unwrap();
        if active.is_some() {
            return httpd::respond_empty(stream, 409); // lost a race
        }
        *active = Some(session.clone());
    }
    log::info!(
        "accepted {} files ({} bytes) from `{}` into `{}`",
        session.files.len(),
        session.total_bytes,
        sender.alias,
        save_dir.display()
    );
    shared.wake.wake(WakeReason::Progress); // transfer screen adopts the session

    let response = PrepareUploadResponse {
        session_id: session.session_id.clone(),
        files: session.tokens(),
    };
    let json = serde_json::to_string(&response).expect("response serializes");
    httpd::respond_json(stream, 200, &json)
}

/// Stream one file into the active session.
fn handle_upload<S: Read + Write>(
    reader: &mut BufReader<S>,
    request: &httpd::Request,
    shared: &Arc<NetShared>,
) -> std::io::Result<()> {
    let (Some(session_id), Some(file_id), Some(token)) = (
        request.query_param("sessionId"),
        request.query_param("fileId"),
        request.query_param("token"),
    ) else {
        return httpd::respond_empty(reader.get_mut(), 400);
    };

    // Clone the Arc out — the lock must not be held while streaming, other
    // files of the same session upload in parallel.
    let session = {
        let active = shared.active.lock().unwrap();
        match &*active {
            Some(s) if s.session_id == session_id => s.clone(),
            _ => return httpd::respond_empty(reader.get_mut(), 403),
        }
    };

    let (file_id, token) = (file_id.to_string(), token.to_string());
    if request.expects_continue {
        httpd::write_continue(reader.get_mut())?;
    }
    let status = {
        let mut body = httpd::body_reader(reader, request);
        session.receive_file(&file_id, &token, &mut body, shared.wake.as_ref())
    };

    if session.is_finished() {
        clear_active(shared, &session.session_id);
        shared.wake.wake(WakeReason::Done);
    }
    httpd::respond_empty(reader.get_mut(), status)
}

/// Sender-side cancel: fail what hasn't arrived, abort what is streaming.
fn handle_cancel<S: Write>(
    stream: &mut S,
    request: &httpd::Request,
    shared: &Arc<NetShared>,
) -> std::io::Result<()> {
    let Some(session_id) = request.query_param("sessionId") else {
        return httpd::respond_empty(stream, 400);
    };
    let session = {
        let active = shared.active.lock().unwrap();
        match &*active {
            Some(s) if s.session_id == session_id => s.clone(),
            // Unknown/finished session: cancel is idempotent.
            _ => return httpd::respond_empty(stream, 200),
        }
    };
    log::info!("`{}` cancelled the transfer", session.peer_alias);
    session.cancel("cancelled by sender");
    clear_active(shared, session_id);
    shared.wake.wake(WakeReason::Done);
    httpd::respond_empty(stream, 200)
}

/// Drop the active session if it is still the one we think it is (another
/// handler may have already replaced or cleared it).
fn clear_active(shared: &Arc<NetShared>, session_id: &str) {
    let mut active = shared.active.lock().unwrap();
    if active.as_ref().is_some_and(|s| s.session_id == session_id) {
        *active = None;
    }
}

/// Read and parse a JSON body up to `cap` bytes, honoring 100-continue.
/// `Ok(None)` = malformed (caller answers 400); over-cap answers 413 inline.
fn read_json_body<S: Read + Write, T: serde::de::DeserializeOwned>(
    reader: &mut BufReader<S>,
    request: &httpd::Request,
    cap: u64,
) -> std::io::Result<Option<T>> {
    if request.content_length > cap {
        httpd::respond_empty(reader.get_mut(), 413)?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "body over cap",
        ));
    }
    if request.expects_continue {
        httpd::write_continue(reader.get_mut())?;
    }
    let mut body = String::new();
    httpd::body_reader(reader, request).read_to_string(&mut body)?;
    Ok(serde_json::from_str::<T>(&body).ok())
}

/// Our info as an HTTP response body: the `announce` flag is a multicast-only
/// field and must not leak into REST responses.
fn public_info(mut me: DeviceInfo) -> DeviceInfo {
    me.announce = None;
    me
}
