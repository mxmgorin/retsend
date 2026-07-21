use serde::{Deserialize, Serialize};

/// Window/display settings (`[display]` in the config).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    /// Request an OpenGL ES context (required on Mali handhelds) instead of
    /// desktop GL. Can be overridden at startup via `RETSEND_GLES=0`.
    pub use_gles: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            use_gles: true,
        }
    }
}
