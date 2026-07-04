use serde::{Deserialize, Serialize};

/// File-transfer behavior (`[transfer]` in the config).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TransferConfig {
    /// Where received files land.
    pub save_dir: String,
    /// Accept incoming transfers without asking ("quick save").
    pub auto_accept: bool,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            save_dir: super::paths::default_save_dir(),
            auto_accept: false,
        }
    }
}
