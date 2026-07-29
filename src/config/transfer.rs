use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// File-transfer behavior (`[transfer]` in the config).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TransferConfig {
    /// Where received files land (the default; see `routes`).
    pub save_dir: String,
    /// Accept incoming transfers without asking ("quick save").
    pub auto_accept: bool,
    /// Extra file-browser roots on top of the auto-detected mount points.
    pub browser_roots: Vec<String>,
    /// Folders and files pinned to the top of the file browser, in the order
    /// shown. Toggled on the device with Y on the row itself.
    pub pinned_paths: Vec<String>,
    /// Folder the last send started from; the browser reopens there instead of
    /// the first root. Written on send, not while browsing — every write hits
    /// the SD card.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_send_dir: String,
    /// Max transfers kept in the History tab; oldest are dropped past this.
    pub history_limit: usize,
    /// Sort received ROMs into the console folders that already exist in
    /// `save_dir` (see [`super::routes`]). Only fills extensions `routes`
    /// doesn't already claim; toggled from Settings → Auto save routes.
    pub auto_routes: bool,
    /// Route received files to per-extension folders, e.g. `gbc = "gb"` or
    /// `png = "/roms/screenshots"`. Extensions match case-insensitively; a
    /// relative folder resolves under `save_dir`, an absolute one is used as
    /// is; unknown extensions fall back to `save_dir`. Takes precedence over
    /// `auto_routes`; editable from Settings → Save routes.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub routes: BTreeMap<String, String>,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            save_dir: super::paths::default_save_dir(),
            auto_accept: false,
            browser_roots: Vec::new(),
            pinned_paths: Vec::new(),
            last_send_dir: String::new(),
            history_limit: crate::transfer::history::DEFAULT_MAX_ENTRIES,
            auto_routes: true,
            routes: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_round_trip() {
        // The `[transfer.routes]` table must serialize after the scalar/array
        // fields (TOML rejects a table before a value in the same parent), and
        // survive a load.
        let cfg = TransferConfig {
            browser_roots: vec!["/roms".into()],
            auto_routes: false,
            routes: BTreeMap::from([("gbc".to_string(), "gb".to_string())]),
            ..Default::default()
        };
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        let back: TransferConfig = toml::from_str(&text).expect("deserialize");
        assert_eq!(back.routes.get("gbc").map(String::as_str), Some("gb"));
        assert_eq!(back.browser_roots, vec!["/roms".to_string()]);
        assert!(!back.auto_routes);
    }

    #[test]
    fn auto_routes_defaults_on_for_configs_written_before_it_existed() {
        let back: TransferConfig = toml::from_str("save_dir = \"/roms\"").expect("deserialize");
        assert!(back.auto_routes);
    }
}
