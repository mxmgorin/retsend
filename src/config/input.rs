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
    pub keys: KeyBindings,
}

/// Per-button key overrides (`[input.keys]`), as SDL key names such as
/// `"Return"`, `"SoftLeft"` or `"0"`. A listed button replaces the layout's keys
/// for it, an empty list unbinds it, and an omitted one keeps the layout's.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyBindings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r1: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu: Option<Vec<String>>,
}

impl KeyBindings {
    pub fn get(&self, button: PadButton) -> Option<&[String]> {
        self.slot(button).as_deref()
    }

    pub fn set(&mut self, button: PadButton, keys: Option<Vec<String>>) {
        *self.slot_mut(button) = keys;
    }

    /// How many buttons are bound by hand rather than by the layout.
    pub fn changed(&self) -> usize {
        PadButton::ALL
            .iter()
            .filter(|&&b| self.get(b).is_some())
            .count()
    }

    fn slot(&self, button: PadButton) -> &Option<Vec<String>> {
        match button {
            PadButton::Up => &self.up,
            PadButton::Down => &self.down,
            PadButton::Left => &self.left,
            PadButton::Right => &self.right,
            PadButton::A => &self.a,
            PadButton::B => &self.b,
            PadButton::X => &self.x,
            PadButton::Y => &self.y,
            PadButton::Start => &self.start,
            PadButton::Select => &self.select,
            PadButton::L1 => &self.l1,
            PadButton::R1 => &self.r1,
            PadButton::Menu => &self.menu,
        }
    }

    fn slot_mut(&mut self, button: PadButton) -> &mut Option<Vec<String>> {
        match button {
            PadButton::Up => &mut self.up,
            PadButton::Down => &mut self.down,
            PadButton::Left => &mut self.left,
            PadButton::Right => &mut self.right,
            PadButton::A => &mut self.a,
            PadButton::B => &mut self.b,
            PadButton::X => &mut self.x,
            PadButton::Y => &mut self.y,
            PadButton::Start => &mut self.start,
            PadButton::Select => &mut self.select,
            PadButton::L1 => &mut self.l1,
            PadButton::R1 => &mut self.r1,
            PadButton::Menu => &mut self.menu,
        }
    }
}

/// A pad button: what a key stands in for, and every input the app knows.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PadButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    Start,
    Select,
    L1,
    R1,
    /// The handheld's MENU: quits, where the launcher leaves that to the app.
    Menu,
}

impl PadButton {
    /// Exhaustive, in on-screen order, so a new variant must place itself.
    pub const ALL: [Self; 13] = [
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
        Self::Start,
        Self::Select,
        Self::L1,
        Self::R1,
        Self::Menu,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::A => "A",
            Self::B => "B",
            Self::X => "X",
            Self::Y => "Y",
            Self::Start => "Start",
            Self::Select => "Select",
            Self::L1 => "L1",
            Self::R1 => "R1",
            Self::Menu => "Menu",
        }
    }

    /// Every screen needs it to be usable, so it must keep a key.
    pub fn is_essential(self) -> bool {
        matches!(
            self,
            Self::Up | Self::Down | Self::Left | Self::Right | Self::A | Self::B
        )
    }
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
            keys: KeyBindings::default(),
        }
    }
}
