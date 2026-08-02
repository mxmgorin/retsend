//! Peers added by hand: parse a typed `ip[:port]`, probe it, and register
//! whatever answers. The way into networks where multicast discovery never
//! delivers — guest wifi, AP isolation, some VPNs. Only the *discovery* is
//! bypassed; the address still has to be routable from here.

use super::protocol::{self, DeviceInfo};
use super::{NetShared, WakeReason};
use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

/// Assumed when the typed address carries no port: LocalSend's default TCP
/// port, the same number as the protocol's fixed multicast one.
const DEFAULT_PORT: u16 = protocol::MULTICAST_PORT;
/// A mistyped address is the common case, so fail fast — four attempts (two
/// schemes, two endpoints) must stay within a few seconds.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
/// Device info responses are tiny; anything bigger is not one.
const MAX_BODY: u64 = 64 * 1024;
/// Tried in order; whichever answers is how we talk to this peer from now on.
const SCHEMES: [&str; 2] = ["https", "http"];

/// Split a typed address into an IP and a port. Accepts `1.2.3.4`,
/// `1.2.3.4:5000`, `[::1]`, `[::1]:5000` and a bare `::1`; the port defaults to
/// [`DEFAULT_PORT`]. The error is the message the user is shown.
pub fn parse_address(raw: &str) -> Result<SocketAddr, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("Enter an IP address".to_string());
    }
    let (host, port) = split_host_port(text)?;
    let ip: IpAddr = host
        .parse()
        .map_err(|_| format!("Not an IP address: {host}"))?;
    Ok(SocketAddr::new(ip, port))
}

fn split_host_port(text: &str) -> Result<(&str, u16), String> {
    // Bracketed IPv6 — the one form where a colon means both things at once.
    if let Some(rest) = text.strip_prefix('[') {
        let (host, tail) = rest
            .split_once(']')
            .ok_or_else(|| format!("Missing ] in {text}"))?;
        let Some(port) = tail.strip_prefix(':') else {
            return if tail.is_empty() {
                Ok((host, DEFAULT_PORT))
            } else {
                Err(format!("Junk after ]: {tail}"))
            };
        };
        return Ok((host, parse_port(port)?));
    }
    match text.split_once(':') {
        // More colons behind the first: an unbracketed IPv6 literal, all host.
        Some((host, port)) if !port.contains(':') => Ok((host, parse_port(port)?)),
        _ => Ok((text, DEFAULT_PORT)),
    }
}

fn parse_port(text: &str) -> Result<u16, String> {
    match text.parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(format!("Not a port: {text}")),
    }
}

/// Probe a typed address off the UI thread and add whatever answers to the
/// registry; the outcome comes back as a toast through [`NetShared::notify`].
pub fn spawn_probe(shared: Arc<NetShared>, addr: SocketAddr) {
    let worker = shared.clone();
    let spawned = std::thread::Builder::new()
        .name("manual-add".into())
        .spawn(move || probe_and_add(&worker, addr));
    if let Err(e) = spawned {
        log::warn!("could not spawn manual-add thread: {e}");
        shared.notify(format!("Can't reach {addr} right now"));
    }
}

fn probe_and_add(shared: &Arc<NetShared>, addr: SocketAddr) {
    let me = { shared.me.lock().unwrap().clone() };
    let Some((mut info, scheme)) = probe(&me, addr) else {
        shared.notify(format!("No device at {addr}"));
        return;
    };
    if info.fingerprint == me.fingerprint {
        shared.notify(format!("{addr} is this device"));
        return;
    }
    // The scheme that answered outranks the announced one: it is the one this
    // address is known to serve.
    info.protocol = Some(scheme.to_string());
    let alias = info.alias.clone();
    log::info!("added `{alias}` at {addr} by hand ({scheme})");
    shared.peers.upsert_manual(info, addr.ip(), addr.port());
    shared.wake.wake(WakeReason::Peers);
    shared.notify(format!("Added {alias}"));
}

/// Ask an address who it is. `/register` first — it also tells the peer about
/// us, exactly like [`super::discovery`] answering an announce — with `/info`
/// as the fallback for anything that turns the register body down.
fn probe(me: &DeviceInfo, addr: SocketAddr) -> Option<(DeviceInfo, &'static str)> {
    for scheme in SCHEMES {
        let base = format!("{scheme}://{addr}{}", protocol::API_PREFIX);
        let agent = super::client::agent(Some(PROBE_TIMEOUT));
        if let Some(info) = register(&agent, &base, me).or_else(|| fetch_info(&agent, &base)) {
            return Some((info, scheme));
        }
    }
    None
}

fn register(agent: &ureq::Agent, base: &str, me: &DeviceInfo) -> Option<DeviceInfo> {
    let body = serde_json::to_string(me).expect("DeviceInfo serializes");
    match agent
        .post(format!("{base}/register"))
        .content_type("application/json")
        .send(body.as_str())
    {
        Ok(mut resp) => read_info(resp.body_mut()),
        Err(e) => {
            log::debug!("register at {base} failed: {e}");
            None
        }
    }
}

fn fetch_info(agent: &ureq::Agent, base: &str) -> Option<DeviceInfo> {
    match agent.get(format!("{base}/info")).call() {
        Ok(mut resp) => read_info(resp.body_mut()),
        Err(e) => {
            log::debug!("info at {base} failed: {e}");
            None
        }
    }
}

fn read_info(body: &mut ureq::Body) -> Option<DeviceInfo> {
    let mut text = String::new();
    body.as_reader()
        .take(MAX_BODY)
        .read_to_string(&mut text)
        .ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_address_takes_the_default_port() {
        let addr = parse_address(" 192.168.1.23 ").unwrap();
        assert_eq!(addr, SocketAddr::from(([192, 168, 1, 23], DEFAULT_PORT)));
    }

    #[test]
    fn explicit_port_wins() {
        let addr = parse_address("192.168.1.23:8080").unwrap();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn ipv6_needs_brackets_only_for_a_port() {
        assert_eq!(
            parse_address("[fe80::1]:5000").unwrap(),
            SocketAddr::new("fe80::1".parse().unwrap(), 5000)
        );
        assert_eq!(
            parse_address("[fe80::1]").unwrap(),
            SocketAddr::new("fe80::1".parse().unwrap(), DEFAULT_PORT)
        );
        assert_eq!(
            parse_address("fe80::1").unwrap(),
            SocketAddr::new("fe80::1".parse().unwrap(), DEFAULT_PORT)
        );
    }

    #[test]
    fn junk_is_rejected_with_a_readable_reason() {
        for bad in [
            "",
            "   ",
            "phone.local",
            "192.168.1",
            "1.2.3.4:",
            "1.2.3.4:0",
        ] {
            assert!(parse_address(bad).is_err(), "{bad} should not parse");
        }
        assert!(parse_address("1.2.3.4:99999").unwrap_err().contains("port"));
        assert!(parse_address("[fe80::1:5000").unwrap_err().contains("]"));
    }
}
