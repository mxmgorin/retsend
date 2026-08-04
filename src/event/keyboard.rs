//! Keyboard controls: the desktop-dev mirror of the gamepad, plus the layout
//! handhelds send when their pad arrives as key presses.

use crate::app::{AppCommand, Direction};
use sdl2::keyboard::Keycode;

/// Which layout the keys arriving from the device follow.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Keymap {
    #[default]
    Desktop,
    /// Its SDL2 offers the pad as a joystick with no gamepad mapping, and sends
    /// keys instead. See [`miyoo_mini`].
    MiyooMini,
}

impl Keymap {
    /// `RETSEND_KEYMAP=miyoo|desktop` wins; else the driver name gives it away.
    pub fn detect(video_driver: &str) -> Self {
        match std::env::var("RETSEND_KEYMAP").as_deref() {
            Ok("miyoo") => Self::MiyooMini,
            Ok("desktop") => Self::Desktop,
            Ok(other) => {
                log::warn!("unknown RETSEND_KEYMAP `{other}`; using the desktop layout");
                Self::Desktop
            }
            Err(_) if video_driver == "mmiyoo" => Self::MiyooMini,
            Err(_) => Self::Desktop,
        }
    }
}

pub fn on_key_down(keymap: Keymap, kc: Keycode, repeat: bool, commands: &mut Vec<AppCommand>) {
    let cmd = match kc {
        Keycode::Up => AppCommand::Nav(Direction::Up),
        Keycode::Down => AppCommand::Nav(Direction::Down),
        Keycode::Left => AppCommand::Nav(Direction::Left),
        Keycode::Right => AppCommand::Nav(Direction::Right),
        Keycode::PageUp => AppCommand::PageUp,
        Keycode::PageDown => AppCommand::PageDown,
        // OS key repeat only drives navigation; a held Enter must not
        // re-confirm and a held Esc must not unwind several screens.
        _ if repeat => return,
        _ => match keymap {
            Keymap::Desktop => match desktop(kc) {
                Some(cmd) => cmd,
                None => return,
            },
            Keymap::MiyooMini => match miyoo_mini(kc) {
                Some(cmd) => cmd,
                None => return,
            },
        },
    };
    commands.push(cmd);
}

fn desktop(kc: Keycode) -> Option<AppCommand> {
    Some(match kc {
        Keycode::Return | Keycode::KpEnter => AppCommand::Confirm,
        Keycode::Escape => AppCommand::Back,
        // Both the pad's label and the key a desktop hand reaches for.
        Keycode::X | Keycode::Backspace => AppCommand::Alt,
        Keycode::F1 => AppCommand::Start,
        Keycode::Tab | Keycode::F5 => AppCommand::ReAnnounce,
        Keycode::Y => AppCommand::TogglePin,
        _ => return None,
    })
}

/// The pad, as keys. MENU is absent: the launcher gives it to the system's own
/// kill helper, as every app there does.
fn miyoo_mini(kc: Keycode) -> Option<AppCommand> {
    Some(match kc {
        Keycode::Space => AppCommand::Confirm,    // A
        Keycode::LCtrl => AppCommand::Back,       // B
        Keycode::LShift => AppCommand::Alt,       // X
        Keycode::LAlt => AppCommand::TogglePin,   // Y
        Keycode::Return => AppCommand::Start,     // Start
        Keycode::RCtrl => AppCommand::ReAnnounce, // Select
        Keycode::E => AppCommand::PageUp,         // L1
        Keycode::T => AppCommand::PageDown,       // R1
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_and_backspace_erase_while_escape_backs_out() {
        let mut commands = Vec::new();
        on_key_down(Keymap::Desktop, Keycode::X, false, &mut commands);
        on_key_down(Keymap::Desktop, Keycode::Backspace, false, &mut commands);
        on_key_down(Keymap::Desktop, Keycode::Escape, false, &mut commands);
        assert_eq!(
            commands,
            vec![AppCommand::Alt, AppCommand::Alt, AppCommand::Back]
        );
    }

    #[test]
    fn y_pins_and_does_not_repeat() {
        let mut commands = Vec::new();
        on_key_down(Keymap::Desktop, Keycode::Y, false, &mut commands);
        assert_eq!(commands, vec![AppCommand::TogglePin]);

        // Holding it must not toggle over and over.
        commands.clear();
        on_key_down(Keymap::Desktop, Keycode::Y, true, &mut commands);
        assert!(commands.is_empty());
    }

    #[test]
    fn the_miyoo_pad_maps_a_to_confirm_and_start_to_start() {
        let mut commands = Vec::new();
        on_key_down(Keymap::MiyooMini, Keycode::Space, false, &mut commands);
        on_key_down(Keymap::MiyooMini, Keycode::LCtrl, false, &mut commands);
        on_key_down(Keymap::MiyooMini, Keycode::Return, false, &mut commands);
        assert_eq!(
            commands,
            vec![AppCommand::Confirm, AppCommand::Back, AppCommand::Start]
        );
    }

    #[test]
    fn arrows_navigate_under_either_layout() {
        for keymap in [Keymap::Desktop, Keymap::MiyooMini] {
            let mut commands = Vec::new();
            on_key_down(keymap, Keycode::Up, false, &mut commands);
            assert_eq!(commands, vec![AppCommand::Nav(Direction::Up)]);
        }
    }
}
