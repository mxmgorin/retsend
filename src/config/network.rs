use serde::{Deserialize, Serialize};

/// Discovery/server settings (`[network]` in the config).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// Preferred TCP port for the HTTP server. When busy (e.g. the official
    /// LocalSend app on the dev machine) the next 9 ports are tried and the
    /// announce carries whichever bound — peers dial from the announce, so
    /// discovery is unaffected. The UDP multicast port stays 53317 regardless:
    /// it's fixed by the protocol.
    pub port: u16,
    /// Seconds between periodic multicast announces.
    pub announce_interval_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            port: 53317,
            announce_interval_secs: 5,
        }
    }
}
