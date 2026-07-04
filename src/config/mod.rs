//! Application configuration: the `config.toml` schema and its file I/O.
//!
//! [`AppConfig`] is the top-level aggregate, one field per `[section]` of the
//! TOML file; each section lives in its own submodule. Same load/template/
//! sanitize contract as retsurf: a missing file yields defaults and writes a
//! template, a malformed file logs and falls back to defaults, and unknown or
//! omitted fields default individually so a partial file is valid.

use serde::{Deserialize, Serialize};

mod device;
mod display;
mod input;
mod network;
mod paths;
mod transfer;

pub use device::DeviceConfig;
pub use display::DisplayConfig;
pub use input::InputConfig;
pub use network::NetworkConfig;
pub use paths::device_scale;
pub use transfer::TransferConfig;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub device: DeviceConfig,
    pub network: NetworkConfig,
    pub transfer: TransferConfig,
    pub display: DisplayConfig,
    pub input: InputConfig,
}

impl AppConfig {
    pub fn load() -> Self {
        let path = paths::config_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str::<Self>(&text) {
                Ok(mut config) => {
                    config.sanitize();
                    log::info!("loaded config from `{path}`");
                    config
                }
                Err(e) => {
                    log::error!("invalid config `{path}`: {e}; using defaults");
                    Self::default()
                }
            },
            Err(_) => {
                let config = Self::default();
                config.write_to(&path, "default config");
                config
            }
        }
    }

    /// Best-effort persist — a failure (read-only SD) degrades to
    /// in-memory-only changes, not a crash.
    pub fn save(&self) {
        self.write_to(&paths::config_path(), "config");
    }

    fn write_to(&self, path: &str, what: &str) {
        match toml::to_string_pretty(self) {
            Ok(text) => match std::fs::write(path, text) {
                Ok(()) => log::info!("wrote {what} to `{path}`"),
                Err(e) => log::warn!("could not write {what} `{path}`: {e}"),
            },
            Err(e) => log::warn!("could not serialize {what}: {e}"),
        }
    }

    /// Clamp hand-edited values into ranges the app can operate with; the GUI
    /// enforces the same ranges, this covers the text editor path.
    fn sanitize(&mut self) {
        fix("display.width", &mut self.display.width, 320, 7680);
        fix("display.height", &mut self.display.height, 240, 4320);
        // Ports below 1024 need root; 0 means "ephemeral" which would break
        // re-announce consistency.
        fix("network.port", &mut self.network.port, 1024, u16::MAX);
        fix(
            "network.announce_interval_secs",
            &mut self.network.announce_interval_secs,
            2,
            600,
        );
        fix_f32("input.deadzone", &mut self.input.deadzone, 0.05, 0.95);
        fix(
            "input.repeat_initial_delay_ms",
            &mut self.input.repeat_initial_delay_ms,
            50,
            2000,
        );
        fix(
            "input.repeat_interval_ms",
            &mut self.input.repeat_interval_ms,
            30,
            1000,
        );
        if self.device.alias.trim().is_empty() {
            log::warn!("config: device.alias is empty; using default");
            self.device.alias = DeviceConfig::default().alias;
        }
    }
}

fn fix<T: PartialOrd + Copy + std::fmt::Display>(name: &str, v: &mut T, min: T, max: T) {
    let before = *v;
    if *v < min {
        *v = min;
    } else if *v > max {
        *v = max;
    }
    if *v != before {
        log::warn!("config: {name} = {before} out of range; using {}", *v);
    }
}

fn fix_f32(name: &str, v: &mut f32, min: f32, max: f32) {
    let before = *v;
    *v = if v.is_finite() {
        v.clamp(min, max)
    } else {
        min
    };
    if before.to_bits() != v.to_bits() {
        log::warn!("config: {name} = {before} out of range; using {}", *v);
    }
}
