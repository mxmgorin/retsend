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

/// The look the UI wears.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// The icon's own dark plate.
    Retsend,
    /// spruceOS's stock look.
    Spruce,
}

impl Theme {
    /// Every theme, in stepping order. Exhaustive: a new variant must be listed.
    pub const ALL: [Theme; 2] = [Theme::Retsend, Theme::Spruce];

    /// This theme's position in [`Self::ALL`].
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|&t| t == self)
            .expect("ALL lists every variant")
    }

    /// The theme `dir` steps along [`Self::ALL`] from this one, wrapping around.
    pub fn step(self, dir: i32) -> Self {
        let count = Self::ALL.len() as i32;
        Self::ALL[(self.index() as i32 + dir).rem_euclid(count) as usize]
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::Retsend => "retsend",
            Theme::Spruce => "spruce",
        }
    }

    /// The theme `name` spells, if any.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// `RETSEND_THEME` when it names a theme, else [`Theme::Retsend`].
impl Default for Theme {
    fn default() -> Self {
        std::env::var("RETSEND_THEME")
            .ok()
            .and_then(|name| Self::from_name(name.trim()))
            .unwrap_or(Theme::Retsend)
    }
}

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
    pub theme: Theme,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            use_gles: true,
            scale: 1.0,
            idle_animation: true,
            theme: Theme::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_spells_a_theme_by_its_name() {
        for theme in Theme::ALL {
            let value = toml::Value::try_from(theme).expect("a unit variant serializes");
            assert_eq!(value.as_str(), Some(theme.name()));
        }
    }

    #[test]
    fn every_theme_is_found_by_its_name() {
        for theme in Theme::ALL {
            assert_eq!(Theme::from_name(theme.name()), Some(theme));
        }
    }

    #[test]
    fn step_walks_every_theme_and_wraps_both_ways() {
        for (at, &theme) in Theme::ALL.iter().enumerate() {
            let count = Theme::ALL.len();
            assert_eq!(theme.step(1), Theme::ALL[(at + 1) % count]);
            assert_eq!(theme.step(-1), Theme::ALL[(at + count - 1) % count]);
        }
    }
}
