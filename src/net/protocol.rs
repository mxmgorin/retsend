//! LocalSend v2.1 wire types (<https://github.com/localsend/protocol>) and the
//! random-token helper. Pure data: everything else in `net` consumes this.
//!
//! One flattened [`DeviceInfo`] with optionals serves every message that
//! carries device identity — the multicast announce, the `/register` body and
//! response, and prepare-upload's `info` are all subsets of it. Input parsing
//! is deliberately lax (real-world LocalSend forks vary); we always emit the
//! full v2.1 shape.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: &str = "2.1";
pub const API_PREFIX: &str = "/api/localsend/v2";
pub const MULTICAST_GROUP: std::net::Ipv4Addr = std::net::Ipv4Addr::new(224, 0, 0, 167);
/// The protocol-fixed UDP discovery port. The TCP port is ours to choose (the
/// announce carries it); this one every implementation must share.
pub const MULTICAST_PORT: u16 = 53317;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub alias: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    pub fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// "http" | "https"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<bool>,
    /// Multicast only: `true` asks receivers to respond (via `/register` or a
    /// multicast reply with `announce: false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announce: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub id: String,
    pub file_name: String,
    pub size: u64,
    pub file_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PrepareUploadRequest {
    pub info: DeviceInfo,
    /// Keyed by file id.
    pub files: BTreeMap<String, FileMeta>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PrepareUploadResponse {
    pub session_id: String,
    /// fileId -> upload token.
    pub files: BTreeMap<String, String>,
}

/// `n_bytes` of OS randomness as lowercase hex. Session ids and per-file
/// tokens use 16 bytes; the per-run fingerprint (HTTP mode: just a random
/// string for self-ignore) uses 32.
pub fn random_token(n_bytes: usize) -> String {
    let mut buf = vec![0u8; n_bytes];
    getrandom::fill(&mut buf).expect("OS randomness unavailable");
    let mut out = String::with_capacity(n_bytes * 2);
    for b in buf {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An announce as the official app (2.x, HTTPS mode) multicasts it.
    const OFFICIAL_ANNOUNCE: &str = r#"{
        "alias": "Nice Orange",
        "version": "2.1",
        "deviceModel": "Samsung",
        "deviceType": "mobile",
        "fingerprint": "e0accd3aax9",
        "port": 53317,
        "protocol": "https",
        "download": true,
        "announce": true
    }"#;

    #[test]
    fn parses_official_announce() {
        let d: DeviceInfo = serde_json::from_str(OFFICIAL_ANNOUNCE).unwrap();
        assert_eq!(d.alias, "Nice Orange");
        assert_eq!(d.version, "2.1");
        assert_eq!(d.device_type.as_deref(), Some("mobile"));
        assert_eq!(d.port, Some(53317));
        assert_eq!(d.protocol.as_deref(), Some("https"));
        assert_eq!(d.announce, Some(true));
    }

    /// Minimal message: only the required fields. Forks omit the rest.
    #[test]
    fn parses_minimal_device_info() {
        let d: DeviceInfo =
            serde_json::from_str(r#"{"alias":"A","version":"2.0","fingerprint":"f"}"#).unwrap();
        assert_eq!(d.alias, "A");
        assert_eq!(d.port, None);
        assert_eq!(d.announce, None);
    }

    /// Unknown extra fields must not fail parsing (forward compatibility).
    #[test]
    fn ignores_unknown_fields() {
        let d: DeviceInfo = serde_json::from_str(
            r#"{"alias":"A","version":"2.1","fingerprint":"f","futureField":42}"#,
        )
        .unwrap();
        assert_eq!(d.alias, "A");
    }

    #[test]
    fn device_info_round_trips() {
        let me = DeviceInfo {
            alias: "LocalSend Retro".into(),
            version: PROTOCOL_VERSION.into(),
            device_model: Some("Retro Handheld".into()),
            device_type: Some("desktop".into()),
            fingerprint: random_token(32),
            port: Some(53317),
            protocol: Some("http".into()),
            download: Some(false),
            announce: Some(true),
        };
        let json = serde_json::to_string(&me).unwrap();
        let back: DeviceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(me, back);
        // Wire names are camelCase, not snake_case.
        assert!(json.contains("\"deviceModel\""));
        assert!(json.contains("\"deviceType\""));
    }

    #[test]
    fn prepare_upload_round_trips() {
        let req = PrepareUploadRequest {
            info: serde_json::from_str(OFFICIAL_ANNOUNCE).unwrap(),
            files: BTreeMap::from([(
                "abc".to_string(),
                FileMeta {
                    id: "abc".into(),
                    file_name: "game.gbc".into(),
                    size: 1024,
                    file_type: "application/octet-stream".into(),
                    sha256: None,
                    preview: None,
                    metadata: None,
                },
            )]),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"fileName\":\"game.gbc\""));
        let back: PrepareUploadRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.files["abc"].file_name, "game.gbc");

        let resp = PrepareUploadResponse {
            session_id: random_token(16),
            files: BTreeMap::from([("abc".to_string(), random_token(16))]),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"sessionId\""));
    }

    #[test]
    fn random_tokens_are_unique_hex() {
        let a = random_token(16);
        let b = random_token(16);
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
