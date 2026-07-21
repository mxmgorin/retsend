//! Path/directory and environment resolution shared across the crate.

/// UI scale factor. The handheld launcher can set `RETSEND_SCALE` for tiny or
/// high-DPI screens; desktop leaves it unset and stays at 1.0. Applied to
/// egui's zoom factor. Clamped to a sane range.
pub fn device_scale() -> f32 {
    std::env::var("RETSEND_SCALE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|s| s.is_finite() && *s > 0.0)
        .map(|s| s.clamp(0.5, 6.0))
        .unwrap_or(1.0)
}

/// The per-user data directory (with a trailing separator) for writable files —
/// currently just the config. `RETSEND_DATA_DIR` overrides it (created on
/// demand — the PortMaster launcher points it into $GAMEDIR); otherwise SDL's
/// `SDL_GetPrefPath` (e.g. `~/.local/share/mxmgorin/retsend/`), which
/// is guaranteed writable and created on demand. Falls back to the working
/// directory if SDL can't provide a pref path.
pub fn data_dir() -> String {
    if let Ok(dir) = std::env::var("RETSEND_DATA_DIR") {
        let dir = dir.trim_end_matches('/');
        if !dir.is_empty() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                log::warn!("could not create RETSEND_DATA_DIR `{dir}`: {e}");
            }
            return format!("{dir}/");
        }
    }
    match sdl2::filesystem::pref_path("mxmgorin", "retsend") {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!("could not resolve preferences directory ({e}); using working directory");
            String::new() // empty prefix => paths resolve relative to the cwd
        }
    }
}

/// Resolve the config file path: `RETSEND_CONFIG` if set, otherwise
/// `config.toml` inside [`data_dir`].
pub(super) fn config_path() -> String {
    if let Ok(path) = std::env::var("RETSEND_CONFIG") {
        return path;
    }
    format!("{}config.toml", data_dir())
}

/// Default directory for received files: `RETSEND_SAVE_DIR` (the PortMaster
/// launcher points it at the device's ROMs root), else `~/Downloads` when it
/// exists, else `received/` inside [`data_dir`].
pub fn default_save_dir() -> String {
    if let Ok(dir) = std::env::var("RETSEND_SAVE_DIR") {
        if !dir.is_empty() {
            return dir;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let downloads = format!("{home}/Downloads");
        if std::path::Path::new(&downloads).is_dir() {
            return downloads;
        }
    }
    format!("{}received", data_dir())
}
