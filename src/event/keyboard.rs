//! Keyboard controls: keys standing in for pad buttons, on a desktop, a phone,
//! or a handheld whose pad arrives as key presses.
//!
//! A [`Layout`] is a preset of key names per [`PadButton`]; `[input.keys]`
//! replaces it button by button, and [`Keymap`] is the two resolved into one
//! keycode lookup. Names are SDL's (`SDL_GetKeyFromName`), which resolve from a
//! static table, so a keymap needs no initialized video subsystem.

use super::{command, is_face};
use crate::app::AppCommand;
use crate::config::input::{KeyBindings, PadButton};
use sdl2::keyboard::Keycode;
use std::collections::HashMap;

/// Which preset the keys arriving from the device follow.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Layout {
    /// A desktop keyboard or a phone's: a keypad's OK arrives as `Select` and
    /// its digits stand in for the pad; letters navigate vim- or WASD-style.
    #[default]
    Desktop,
    /// Its SDL2 sends the pad as keys. `menu_quits`: the launcher keeps no kill
    /// helper of its own, so MENU is the app's.
    MiyooMini { menu_quits: bool },
}

impl Layout {
    /// `RETSEND_KEYMAP=miyoo|desktop` wins; else the driver name gives it away.
    pub fn detect(video_driver: &str) -> Self {
        let miyoo = Self::MiyooMini {
            menu_quits: std::env::var_os("RETSEND_MENU_QUIT").is_some_and(|v| v != "0"),
        };
        match std::env::var("RETSEND_KEYMAP").as_deref() {
            Ok("miyoo") => miyoo,
            Ok("desktop") => Self::Desktop,
            Ok(other) => {
                log::warn!("unknown RETSEND_KEYMAP `{other}`; using the desktop layout");
                Self::Desktop
            }
            Err(_) if video_driver == "mmiyoo" => miyoo,
            Err(_) => Self::Desktop,
        }
    }

    /// The preset's keys for `button`.
    fn keys(self, button: PadButton) -> &'static [&'static str] {
        match self {
            Self::Desktop => match button {
                PadButton::Up => &["Up", "2", "K", "W"],
                PadButton::Down => &["Down", "8", "J", "S"],
                PadButton::Left => &["Left", "4", "H", "A"],
                PadButton::Right => &["Right", "6", "L", "D"],
                PadButton::A => &["Return", "Keypad Enter", "V", "Select"],
                // AC Back: Android's Back, trapped into a key event.
                PadButton::B => &["Escape", "AC Back", "Z"],
                PadButton::X => &["X", "Backspace", "7"],
                PadButton::Y => &["Y", "C", "9"],
                // Shift and Alt stand in for the Tab and F-keys a QWERTY phone
                // lacks; soft keys sit as Select and Start, the left one as Menu.
                PadButton::Start => &["Space", "Left Alt", "Right Alt", "SoftRight", "Call", "5"],
                PadButton::Select => &["Left Shift", "Right Shift", "Tab", "SoftLeft", "Menu", "0"],
                PadButton::L1 => &["PageUp", "1", "Q"],
                PadButton::R1 => &["PageDown", "3", "E"],
                PadButton::Menu => &[],
            },
            // MENU is the launcher's unless it keeps no kill helper (Allium).
            Self::MiyooMini { menu_quits } => match button {
                PadButton::Up => &["Up"],
                PadButton::Down => &["Down"],
                PadButton::Left => &["Left"],
                PadButton::Right => &["Right"],
                PadButton::A => &["Space"],
                PadButton::B => &["Left Ctrl"],
                PadButton::X => &["Left Shift"],
                PadButton::Y => &["Left Alt"],
                PadButton::Start => &["Return"],
                PadButton::Select => &["Right Ctrl"],
                PadButton::L1 => &["E", "PageUp"],
                PadButton::R1 => &["T", "PageDown"],
                PadButton::Menu if menu_quits => &["Escape"],
                PadButton::Menu => &[],
            },
        }
    }

    /// Whether the keys are a pad's own buttons: its face keys follow the
    /// swap, and there is no keyboard to rebind.
    pub fn is_pad(self) -> bool {
        matches!(self, Self::MiyooMini { .. })
    }
}

/// Only movement follows OS key repeat: a held Enter must not re-confirm.
fn repeats(button: PadButton) -> bool {
    matches!(
        button,
        PadButton::Up
            | PadButton::Down
            | PadButton::Left
            | PadButton::Right
            | PadButton::L1
            | PadButton::R1
    )
}

#[derive(Copy, Clone)]
struct Binding {
    button: PadButton,
    /// Follows the A/B and X/Y swap.
    face: bool,
}

/// A layout with its `[input.keys]` overrides applied, keyed by keycode.
pub struct Keymap {
    layout: Layout,
    bindings: HashMap<Keycode, Binding>,
    /// The next key press is reported as [`AppCommand::BindKey`] instead.
    capturing: bool,
}

impl Keymap {
    pub fn new(layout: Layout, overrides: &KeyBindings) -> Self {
        let keymap = Self {
            layout,
            bindings: resolve(layout, overrides),
            capturing: false,
        };
        keymap.publish_hints(overrides);
        keymap
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn set_overrides(&mut self, overrides: &KeyBindings) {
        self.bindings = resolve(self.layout, overrides);
        self.publish_hints(overrides);
    }

    fn owner(&self, kc: Keycode) -> Option<PadButton> {
        self.bindings.get(&kc).map(|b| b.button)
    }

    fn publish_hints(&self, overrides: &KeyBindings) {
        let first = PadButton::ALL.map(|button| {
            effective_keys(self.layout, overrides, button)
                .iter()
                .filter_map(|name| Keycode::from_name(name))
                .find(|&kc| self.owner(kc) == Some(button))
        });
        super::hints::rebind(first, |kc| self.owner(kc));
    }

    /// Arm or disarm capture. One-shot: the captured press disarms it.
    pub fn capture(&mut self, on: bool) {
        self.capturing = on;
    }

    pub fn on_key_down(&mut self, kc: Keycode, repeat: bool, commands: &mut Vec<AppCommand>) {
        if self.capturing && !repeat {
            self.capturing = false;
            commands.push(if self.cancels_capture(kc) {
                AppCommand::Back
            } else {
                AppCommand::BindKey(kc)
            });
            return;
        }
        let Some(binding) = self.bindings.get(&kc) else {
            return;
        };
        if repeat && !repeats(binding.button) {
            return;
        }
        if self.layout.is_pad() {
            super::hints::pad_used();
        } else {
            super::hints::key_used(binding.button, kc);
        }
        commands.extend(self.command(kc));
    }

    /// Esc and Android's Back always leave a capture, so it can be cancelled
    /// whatever was bound. A pad sent as keys has neither: its B does.
    fn cancels_capture(&self, kc: Keycode) -> bool {
        kc == Keycode::ESCAPE
            || kc == Keycode::AC_BACK
            || (self.layout.is_pad() && self.command(kc) == Some(AppCommand::Back))
    }

    fn command(&self, kc: Keycode) -> Option<AppCommand> {
        let binding = self.bindings.get(&kc)?;
        let cmd = command(binding.button);
        Some(if binding.face {
            super::swap_face(cmd)
        } else {
            cmd
        })
    }
}

/// The keys `button` answers to: its override, else the layout's less those
/// another button's override has taken.
pub fn effective_keys(layout: Layout, overrides: &KeyBindings, button: PadButton) -> Vec<String> {
    if let Some(keys) = overrides.get(button) {
        return keys.to_vec();
    }
    let taken: Vec<Keycode> = PadButton::ALL
        .into_iter()
        .filter_map(|a| overrides.get(a))
        .flatten()
        .filter_map(|name| Keycode::from_name(name))
        .collect();
    layout
        .keys(button)
        .iter()
        .filter(|name| Keycode::from_name(name).is_none_or(|kc| !taken.contains(&kc)))
        .map(|name| name.to_string())
        .collect()
}

/// Bind `kc` to `button` on top of its keys, taking it from whichever button
/// had it. Refused, naming the button, when that would leave an essential one
/// with no key.
pub fn bind_key(
    layout: Layout,
    overrides: &mut KeyBindings,
    button: PadButton,
    kc: Keycode,
) -> Result<(), PadButton> {
    let is_kc = |name: &String| Keycode::from_name(name) == Some(kc);
    let mut taken = Vec::new();
    for other in PadButton::ALL.into_iter().filter(|&a| a != button) {
        let keys = effective_keys(layout, overrides, other);
        if keys.iter().any(is_kc) {
            let rest: Vec<String> = keys.into_iter().filter(|k| !is_kc(k)).collect();
            if rest.is_empty() && other.is_essential() {
                return Err(other);
            }
            taken.push((other, rest));
        }
    }
    for (other, rest) in taken {
        overrides.set(other, Some(rest));
    }
    let mut keys = effective_keys(layout, overrides, button);
    if !keys.iter().any(is_kc) {
        keys.push(kc.name());
    }
    overrides.set(button, Some(keys));
    Ok(())
}

/// A key's name as the screen shows it: what the key is labelled, where
/// SDL's name says something else or reads as a pad button.
pub fn key_label(name: &str) -> &str {
    match name {
        "Return" => "Enter",
        "Escape" => "Esc",
        "Keypad Enter" => "Num Enter",
        // A keypad phone's OK, not the pad's Select.
        "Select" => "OK",
        "AC Back" => "Back",
        "SoftLeft" => "Left soft key",
        "SoftRight" => "Right soft key",
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "Left Shift" => "L Shift",
        "Right Shift" => "R Shift",
        "Left Alt" => "L Alt",
        "Right Alt" => "R Alt",
        other => other,
    }
}

fn resolve(layout: Layout, overrides: &KeyBindings) -> HashMap<Keycode, Binding> {
    let mut bindings = HashMap::new();
    // Presets first, so an override that takes one of their keys wins it.
    for button in PadButton::ALL {
        if overrides.get(button).is_none() {
            let face = layout.is_pad() && is_face(button);
            for name in layout.keys(button) {
                bind(&mut bindings, name, Binding { button, face });
            }
        }
    }
    for button in PadButton::ALL {
        for name in overrides.get(button).unwrap_or_default() {
            bind(
                &mut bindings,
                name,
                Binding {
                    button,
                    face: false,
                },
            );
        }
    }
    bindings
}

fn bind(bindings: &mut HashMap<Keycode, Binding>, name: &str, binding: Binding) {
    match Keycode::from_name(name) {
        Some(kc) => {
            bindings.insert(kc, binding);
        }
        None => log::warn!(
            "config: unknown key `{name}` for {:?}; ignored",
            binding.button
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Direction;

    const DESKTOP: Layout = Layout::Desktop;
    const MIYOO: Layout = Layout::MiyooMini { menu_quits: false };

    /// The swap is read live and process-wide: a test that sees it holds this.
    static SWAP: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn press(keymap: &mut Keymap, keys: &[Keycode]) -> Vec<AppCommand> {
        let mut commands = Vec::new();
        for &kc in keys {
            keymap.on_key_down(kc, false, &mut commands);
        }
        commands
    }

    fn key(name: &str) -> Keycode {
        Keycode::from_name(name).expect("test key names are SDL's")
    }

    #[test]
    fn every_preset_key_name_resolves() {
        for layout in [DESKTOP, Layout::MiyooMini { menu_quits: true }] {
            for button in PadButton::ALL {
                for name in layout.keys(button) {
                    assert!(Keycode::from_name(name).is_some(), "{layout:?}: `{name}`");
                }
            }
        }
    }

    #[test]
    fn a_qwerty_phone_has_every_button_without_tab_or_f_keys() {
        let mut keymap = Keymap::new(DESKTOP, &KeyBindings::default());
        assert_eq!(
            press(
                &mut keymap,
                &[
                    Keycode::Return,
                    key("AC Back"),
                    Keycode::Backspace,
                    Keycode::C,
                    Keycode::Space,
                    Keycode::LAlt,
                    Keycode::LShift,
                ]
            ),
            vec![
                AppCommand::Confirm,
                AppCommand::Back,
                AppCommand::Alt,
                AppCommand::TogglePin,
                AppCommand::Start,
                AppCommand::Start,
                AppCommand::ReAnnounce,
            ]
        );
    }

    #[test]
    fn y_pins_and_does_not_repeat() {
        let mut keymap = Keymap::new(DESKTOP, &KeyBindings::default());
        assert_eq!(
            press(&mut keymap, &[Keycode::C]),
            vec![AppCommand::TogglePin]
        );

        // Holding it must not toggle over and over.
        let mut commands = Vec::new();
        keymap.on_key_down(Keycode::C, true, &mut commands);
        assert!(commands.is_empty());
    }

    #[test]
    fn a_keypad_phone_has_every_button_without_a_pad() {
        let mut keymap = Keymap::new(DESKTOP, &KeyBindings::default());
        assert_eq!(
            press(
                &mut keymap,
                &[
                    key("Select"),
                    key("AC Back"),
                    key("7"),
                    key("9"),
                    key("SoftRight"),
                    key("Call"),
                    key("5"),
                    key("SoftLeft"),
                    key("Menu"),
                    key("0"),
                    key("1"),
                    key("3"),
                    key("2"),
                    key("8"),
                    key("4"),
                    key("6"),
                ]
            ),
            vec![
                AppCommand::Confirm,
                AppCommand::Back,
                AppCommand::Alt,
                AppCommand::TogglePin,
                AppCommand::Start,
                AppCommand::Start,
                AppCommand::Start,
                AppCommand::ReAnnounce,
                AppCommand::ReAnnounce,
                AppCommand::ReAnnounce,
                AppCommand::PageUp,
                AppCommand::PageDown,
                AppCommand::Nav(Direction::Up),
                AppCommand::Nav(Direction::Down),
                AppCommand::Nav(Direction::Left),
                AppCommand::Nav(Direction::Right),
            ]
        );
    }

    #[test]
    fn letters_navigate_wasd_style_too() {
        let mut keymap = Keymap::new(DESKTOP, &KeyBindings::default());
        assert_eq!(
            press(&mut keymap, &[key("W"), key("A"), key("S"), key("D")]),
            vec![
                AppCommand::Nav(Direction::Up),
                AppCommand::Nav(Direction::Left),
                AppCommand::Nav(Direction::Down),
                AppCommand::Nav(Direction::Right),
            ]
        );
    }

    #[test]
    fn a_qwerty_phone_navigates_vim_style() {
        let mut keymap = Keymap::new(DESKTOP, &KeyBindings::default());
        assert_eq!(
            press(
                &mut keymap,
                &[key("H"), key("J"), key("K"), key("L"), key("Q"), key("E"),]
            ),
            vec![
                AppCommand::Nav(Direction::Left),
                AppCommand::Nav(Direction::Down),
                AppCommand::Nav(Direction::Up),
                AppCommand::Nav(Direction::Right),
                AppCommand::PageUp,
                AppCommand::PageDown,
            ]
        );
    }

    /// Z X C V read B X Y A, so the X key stays X.
    #[test]
    fn z_x_c_v_press_the_face_buttons() {
        let mut keymap = Keymap::new(DESKTOP, &KeyBindings::default());
        let face = [
            AppCommand::Back,
            AppCommand::Alt,
            AppCommand::TogglePin,
            AppCommand::Confirm,
        ];
        assert_eq!(
            press(&mut keymap, &[key("Z"), key("X"), key("C"), key("V")]),
            face
        );
    }

    #[test]
    fn the_miyoo_pad_maps_a_to_confirm_and_start_to_start() {
        let _swap = SWAP.lock().unwrap_or_else(|e| e.into_inner());
        let mut keymap = Keymap::new(MIYOO, &KeyBindings::default());
        assert_eq!(
            press(
                &mut keymap,
                &[Keycode::Space, Keycode::LCtrl, Keycode::Return]
            ),
            vec![AppCommand::Confirm, AppCommand::Back, AppCommand::Start]
        );
    }

    #[test]
    fn swapped_the_miyoo_pad_confirms_on_b_and_pins_on_x() {
        let _swap = SWAP.lock().unwrap_or_else(|e| e.into_inner());
        let mut keymap = Keymap::new(MIYOO, &KeyBindings::default());
        crate::config::input::set_swap_abxy(true);
        let commands = press(
            &mut keymap,
            &[
                Keycode::Space,
                Keycode::LCtrl,
                Keycode::LShift,
                Keycode::LAlt,
                Keycode::Return,
            ],
        );
        crate::config::input::set_swap_abxy(false);
        assert_eq!(
            commands,
            vec![
                AppCommand::Back,
                AppCommand::Confirm,
                AppCommand::TogglePin,
                AppCommand::Alt,
                AppCommand::Start,
            ]
        );
    }

    /// MENU is the launcher's key until a launcher says otherwise, and it is
    /// never the desktop's Escape, which backs out of a screen.
    #[test]
    fn menu_quits_only_where_the_launcher_hands_it_over() {
        let mut kept = Keymap::new(MIYOO, &KeyBindings::default());
        assert!(press(&mut kept, &[Keycode::Escape]).is_empty());

        let mut handed = Keymap::new(
            Layout::MiyooMini { menu_quits: true },
            &KeyBindings::default(),
        );
        assert_eq!(
            press(&mut handed, &[Keycode::Escape]),
            vec![AppCommand::Shutdown]
        );
    }

    #[test]
    fn arrows_navigate_under_either_layout() {
        for layout in [DESKTOP, Layout::MiyooMini { menu_quits: true }] {
            let mut keymap = Keymap::new(layout, &KeyBindings::default());
            assert_eq!(
                press(&mut keymap, &[Keycode::Up]),
                vec![AppCommand::Nav(Direction::Up)]
            );
        }
    }

    #[test]
    fn an_override_replaces_the_buttons_keys_and_takes_its_key_from_another() {
        let keys = KeyBindings {
            a: Some(vec!["Return".into()]),
            y: Some(vec!["Escape".into()]),
            ..KeyBindings::default()
        };
        let mut keymap = Keymap::new(DESKTOP, &keys);
        assert_eq!(
            press(&mut keymap, &[Keycode::Return, Keycode::V, Keycode::Escape]),
            vec![AppCommand::Confirm, AppCommand::TogglePin]
        );
        // The rest of B's keys stay.
        assert_eq!(
            press(&mut keymap, &[key("AC Back")]),
            vec![AppCommand::Back]
        );
    }

    #[test]
    fn an_empty_list_unbinds_and_an_unknown_name_is_skipped() {
        let keys = KeyBindings {
            select: Some(vec![]),
            start: Some(vec!["NoSuchKey".into(), "F2".into()]),
            ..KeyBindings::default()
        };
        let mut keymap = Keymap::new(DESKTOP, &keys);
        assert!(press(&mut keymap, &[Keycode::Tab, Keycode::Space]).is_empty());
        assert_eq!(press(&mut keymap, &[Keycode::F2]), vec![AppCommand::Start]);
    }

    /// Bound by the user, a Miyoo key means what it says: the swap is for
    /// the preset's pad letters only.
    #[test]
    fn an_overridden_face_key_ignores_the_swap() {
        let keys = KeyBindings {
            a: Some(vec!["Space".into()]),
            ..KeyBindings::default()
        };
        let keymap = Keymap::new(MIYOO, &keys);
        assert!(!keymap.bindings[&Keycode::Space].face);
        assert!(keymap.bindings[&Keycode::LCtrl].face);
    }

    #[test]
    fn overrides_round_trip_through_the_config_file() {
        let text = "[input.keys]\na = [\"Select\", \"5\"]\n";
        let config: crate::config::AppConfig = toml::from_str(text).expect("valid toml");
        assert_eq!(
            config.input.keys.a.as_deref(),
            Some(&["Select".to_string(), "5".to_string()][..])
        );
        assert!(config.input.keys.b.is_none());
    }

    #[test]
    fn a_captured_key_is_reported_once_and_back_cancels() {
        let mut keymap = Keymap::new(DESKTOP, &KeyBindings::default());
        keymap.capture(true);
        assert_eq!(
            press(&mut keymap, &[key("5"), key("5")]),
            vec![AppCommand::BindKey(key("5")), AppCommand::Start]
        );

        keymap.capture(true);
        assert_eq!(
            press(&mut keymap, &[Keycode::Escape, Keycode::X]),
            vec![AppCommand::Back, AppCommand::Alt]
        );

        // Bound to B, Z is still a key to capture.
        keymap.capture(true);
        assert_eq!(
            press(&mut keymap, &[Keycode::Z]),
            vec![AppCommand::BindKey(Keycode::Z)]
        );
    }

    #[test]
    fn the_miyoo_pads_b_cancels_a_capture() {
        let mut keymap = Keymap::new(MIYOO, &KeyBindings::default());
        keymap.capture(true);
        let _swap = SWAP.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            press(&mut keymap, &[Keycode::LCtrl]),
            vec![AppCommand::Back]
        );
    }

    #[test]
    fn binding_adds_the_key_and_takes_it_from_its_old_button() {
        let mut keys = KeyBindings::default();
        bind_key(DESKTOP, &mut keys, PadButton::A, key("5")).expect("Start keeps Space");
        assert_eq!(
            effective_keys(DESKTOP, &keys, PadButton::A),
            ["Return", "Keypad Enter", "V", "Select", "5"]
        );
        assert_eq!(
            effective_keys(DESKTOP, &keys, PadButton::Start),
            ["Space", "Left Alt", "Right Alt", "SoftRight", "Call"]
        );
        let mut keymap = Keymap::new(DESKTOP, &keys);
        assert_eq!(press(&mut keymap, &[key("5")]), vec![AppCommand::Confirm]);

        // Reset, Start still lists only what it answers to.
        keys.set(PadButton::Start, None);
        assert_eq!(
            effective_keys(DESKTOP, &keys, PadButton::Start),
            ["Space", "Left Alt", "Right Alt", "SoftRight", "Call"]
        );
    }

    #[test]
    fn binding_refuses_to_leave_an_essential_button_keyless() {
        let mut keys = KeyBindings {
            b: Some(vec!["Escape".into()]),
            ..KeyBindings::default()
        };
        assert_eq!(
            bind_key(DESKTOP, &mut keys, PadButton::Y, Keycode::Escape),
            Err(PadButton::B)
        );
        assert!(keys.y.is_none(), "a refusal changes nothing");
    }

    #[test]
    fn a_bound_key_name_resolves_back_to_the_key() {
        for name in [
            "Return",
            "Keypad Enter",
            "AC Back",
            "SoftLeft",
            "Select",
            "0",
            "X",
        ] {
            let kc = key(name);
            assert_eq!(Keycode::from_name(&kc.name()), Some(kc), "`{name}`");
        }
    }
}
