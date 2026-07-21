//! The device's TLS identity for the protocol's HTTPS mode: a self-signed
//! certificate generated once and persisted in the data dir. Its SHA-256 is
//! the announce fingerprint — LocalSend's trust model is "same fingerprint =
//! same device" (peers remember it), not CA validation, which is also why it
//! must survive restarts unlike the random HTTP-mode fingerprint.

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;

const CERT_FILE: &str = "identity.crt"; // certificate, DER
const KEY_FILE: &str = "identity.key"; // private key, PKCS#8 DER

pub struct Identity {
    pub server_config: Arc<rustls::ServerConfig>,
    /// Lowercase hex SHA-256 of the certificate DER.
    pub fingerprint: String,
}

/// Install ring as the process-wide rustls provider (idempotent). One
/// provider serves both sides: our https server and ureq's client TLS.
pub fn install_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Load the persisted identity or generate a fresh one. Any problem with the
/// stored pair (corruption, rustls rejection) falls through to regeneration —
/// peers will re-prompt for the "new" device, which beats not starting.
pub fn load_or_create(data_dir: &Path) -> Result<Identity, String> {
    if let Some(identity) = load(data_dir) {
        return Ok(identity);
    }
    let certified = rcgen::generate_simple_self_signed(vec!["localsend-retro".to_string()])
        .map_err(|e| format!("generate certificate: {e}"))?;
    let cert = certified.cert.der().to_vec();
    let key = certified.signing_key.serialize_der();
    // Best-effort persistence: a read-only data dir means a fresh fingerprint
    // per run (HTTP-mode behavior), not a startup failure.
    if let Err(e) = std::fs::write(data_dir.join(CERT_FILE), &cert) {
        log::warn!("could not persist {CERT_FILE}: {e}");
    }
    if let Err(e) = std::fs::write(data_dir.join(KEY_FILE), &key) {
        log::warn!("could not persist {KEY_FILE}: {e}");
    }
    log::info!("generated a new TLS identity");
    build(cert, key)
}

fn load(data_dir: &Path) -> Option<Identity> {
    let cert = std::fs::read(data_dir.join(CERT_FILE)).ok()?;
    let key = std::fs::read(data_dir.join(KEY_FILE)).ok()?;
    match build(cert, key) {
        Ok(identity) => Some(identity),
        Err(e) => {
            log::warn!("stored TLS identity unusable ({e}); regenerating");
            None
        }
    }
}

fn build(cert: Vec<u8>, key: Vec<u8>) -> Result<Identity, String> {
    let fingerprint = fingerprint_hex(&cert);
    let cert = CertificateDer::from(cert);
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key));
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(|e| format!("build server config: {e}"))?;
    Ok(Identity {
        server_config: Arc::new(config),
        fingerprint,
    })
}

fn fingerprint_hex(cert_der: &[u8]) -> String {
    let digest = Sha256::digest(cert_der);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_persists_across_loads() {
        let dir = std::env::temp_dir().join(format!(
            "lsretro-tls-{}",
            crate::net::protocol::random_token(4)
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let first = load_or_create(&dir).unwrap();
        let second = load_or_create(&dir).unwrap();
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.fingerprint.len(), 64);

        // A wiped dir yields a new identity.
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let third = load_or_create(&dir).unwrap();
        assert_ne!(first.fingerprint, third.fingerprint);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
