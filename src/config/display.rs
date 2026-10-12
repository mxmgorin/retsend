use super::Range;
use serde::{Deserialize, Serialize};

/// How far the scale may be pushed either side of the fit. Relative, not
/// absolute: the app already sizes itself to the panel, and a factor that suits
/// a handheld would be wrong on a desktop.
pub const SCALE: Range = Range {
    min: 0.6,
    max: 1.6,
    step: 0.05,
};

/// Window/display settings (`[display]` in the config).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    /// Request an OpenGL ES context (required on Mali handhelds) instead of
    /// desktop GL. Can be overridden at startup via `RETSEND_GLES=0`.
    pub use_gles: bool,
    /// Over the scale the panel's own size asks for, so one setting means the
    /// same thing on every device.
    pub scale: f32,
    /// The landing screens' orbit turns while waiting; off spares a software
    /// rasterizer the frames.
    pub idle_animation: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            use_gles: true,
            scale: 1.0,
            idle_animation: true,
        }
    }
}
