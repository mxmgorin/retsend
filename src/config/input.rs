use serde::{Deserialize, Serialize};

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
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            deadzone: 0.5,
            repeat_initial_delay_ms: 350,
            repeat_interval_ms: 110,
        }
    }
}
