//! The HTTP accept loop and endpoint routing. M1 surface: `/register` (peers
//! introducing themselves over TCP) and `/info` (debug/smoke-test). The
//! transfer endpoints (`prepare-upload`, `upload`, `cancel`) land with M2 —
//! until then they answer 501 so a probing client sees a live but incomplete
//! peer rather than a black hole.

use super::httpd;
use super::protocol::{self, DeviceInfo};
use super::{NetShared, WakeReason};
use std::io::{BufReader, Read};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// JSON bodies (`/register`) are tiny; cap far above any real one.
const MAX_JSON_BODY: u64 = 64 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(30);

/// Bind the preferred port, falling back through the next 9 (the official app
/// may hold 53317 on a dev machine). Returns the accept-loop handle and the
/// port actually bound — the caller stores it into `me.port` before discovery
/// starts, so announces always carry a dialable port.
pub fn spawn(
    shared: Arc<NetShared>,
    preferred_port: u16,
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
            let spawned = std::thread::Builder::new()
                .name(format!("http-{peer_addr}"))
                .spawn(move || handle_connection(stream, &shared));
            if let Err(e) = spawned {
                log::warn!("could not spawn connection thread: {e}");
            }
        })?;

    Ok((handle, port))
}

fn handle_connection(stream: TcpStream, shared: &Arc<NetShared>) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let peer_ip = match stream.peer_addr() {
        Ok(a) => a.ip(),
        Err(_) => return, // connection already gone
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
        ("POST", Some("/prepare-upload" | "/upload" | "/cancel" | "/prepare-download")) => {
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
fn handle_register(
    reader: &mut BufReader<TcpStream>,
    request: &httpd::Request,
    shared: &Arc<NetShared>,
    peer_ip: std::net::IpAddr,
) -> std::io::Result<()> {
    if request.content_length > MAX_JSON_BODY {
        return httpd::respond_empty(reader.get_mut(), 413);
    }
    if request.expects_continue {
        httpd::write_continue(reader.get_mut())?;
    }
    let mut body = String::new();
    httpd::body_reader(reader, request).read_to_string(&mut body)?;
    let Ok(info) = serde_json::from_str::<DeviceInfo>(&body) else {
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

/// Our info as an HTTP response body: the `announce` flag is a multicast-only
/// field and must not leak into REST responses.
fn public_info(mut me: DeviceInfo) -> DeviceInfo {
    me.announce = None;
    me
}
