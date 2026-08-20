use serde::{Deserialize, Serialize};

/// Who we are on the network (`[device]` in the config).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    /// Name shown on other devices' radars. Defaults to the hostname.
    pub alias: String,
    /// Shown under the alias in the official app's device list.
    pub device_model: String,
    /// LocalSend device type (`mobile`/`desktop`/`web`/`headless`/`server`) —
    /// UI-only on the other side, picks the icon there.
    pub device_type: String,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            alias: default_alias(),
            device_model: "Retro Handheld".to_string(),
            device_type: default_device_type().to_string(),
        }
    }
}

/// `RETSEND_ALIAS` (Android has no hostname; the activity passes the device
/// model), else the hostname, else a recognizable fallback.
fn default_alias() -> String {
    if let Ok(alias) = std::env::var("RETSEND_ALIAS") {
        let alias = alias.trim();
        if !alias.is_empty() {
            return alias.to_string();
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "retsend".to_string())
}

/// The protocol has no handheld type, so they ride as desktops; Android is a
/// mobile.
fn default_device_type() -> &'static str {
    if cfg!(target_os = "android") {
        "mobile"
    } else {
        "desktop"
    }
}
