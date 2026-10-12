use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Input tunables (`[input]` in the config).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    /// Analog stick dead zone (0..1); beyond it the left stick acts as a D-pad.
    pub deadzone: f32,
    /// Held D-pad/stick: delay before navigation starts repeating, in ms.
    pub repeat_initial_delay_ms: u64,
    /// Held D-pad/stick: interval between repeats once started, in ms.
    pub repeat_interval_ms: u64,
    /// A and B, X and Y trade places: the hints then name the buttons as a
    /// Nintendo-style pad prints them, A east and X north.
    pub swap_abxy: bool,
}

/// The live [`InputConfig::swap_abxy`]: the setting toggles at runtime, and
/// input routing and the hints read it without a config in hand.
static SWAP_ABXY: AtomicBool = AtomicBool::new(false);

pub fn swap_abxy() -> bool {
    SWAP_ABXY.load(Ordering::Relaxed)
}

pub fn set_swap_abxy(on: bool) {
    SWAP_ABXY.store(on, Ordering::Relaxed);
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.5,
            repeat_initial_delay_ms: 350,
            repeat_interval_ms: 110,
            swap_abxy: false,
        }
    }
}
