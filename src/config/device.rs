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
            device_type: "desktop".to_string(),
        }
    }
}

/// Hostname when readable (Linux-only targets), else a recognizable fallback.
fn default_alias() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "LocalSend Retro".to_string())
}
