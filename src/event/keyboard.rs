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
    /// keys instead. See [`miyoo_mini`]. `menu_quits` is the launcher saying it
    /// keeps no kill helper of its own, which leaves MENU to the app.
    MiyooMini { menu_quits: bool },
}

impl Keymap {
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
            Keymap::MiyooMini { menu_quits } => match miyoo_mini(kc, menu_quits) {
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
        // AcBack is Android's hardware/gesture Back, trapped into a key event by
        // the hint `run_app` sets there.
        Keycode::Escape | Keycode::AcBack => AppCommand::Back,
        // Both the pad's label and the key a desktop hand reaches for.
        Keycode::X | Keycode::Backspace => AppCommand::Alt,
        Keycode::F1 => AppCommand::Start,
        Keycode::Tab | Keycode::F5 => AppCommand::ReAnnounce,
        Keycode::Y => AppCommand::TogglePin,
        _ => return None,
    })
}

/// The pad, as keys. MENU belongs to the launcher — OnionOS gives it to the
/// system's own kill helper, as every app there does — and is the app's to
/// answer only where the launcher keeps none (Allium).
fn miyoo_mini(kc: Keycode, menu_quits: bool) -> Option<AppCommand> {
    Some(match kc {
        // MENU, where the launcher has handed it over.
        Keycode::Escape if menu_quits => AppCommand::Shutdown,
        Keycode::Space => super::swap_face(AppCommand::Confirm), // A
        Keycode::LCtrl => super::swap_face(AppCommand::Back),    // B
        Keycode::LShift => super::swap_face(AppCommand::Alt),    // X
        Keycode::LAlt => super::swap_face(AppCommand::TogglePin), // Y
        Keycode::Return => AppCommand::Start,                    // Start
        Keycode::RCtrl => AppCommand::ReAnnounce,                // Select
        Keycode::E => AppCommand::PageUp,                        // L1
        Keycode::T => AppCommand::PageDown,                      // R1
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
        let miyoo = Keymap::MiyooMini { menu_quits: false };
        let mut commands = Vec::new();
        on_key_down(miyoo, Keycode::Space, false, &mut commands);
        on_key_down(miyoo, Keycode::LCtrl, false, &mut commands);
        on_key_down(miyoo, Keycode::Return, false, &mut commands);
        assert_eq!(
            commands,
            vec![AppCommand::Confirm, AppCommand::Back, AppCommand::Start]
        );
    }

    /// The swap is read live, so the test restores it: the tests share one
    /// process.
    #[test]
    fn swapped_the_miyoo_pad_confirms_on_b_and_pins_on_x() {
        let miyoo = Keymap::MiyooMini { menu_quits: false };
        crate::config::input::set_swap_abxy(true);
        let mut commands = Vec::new();
        on_key_down(miyoo, Keycode::Space, false, &mut commands);
        on_key_down(miyoo, Keycode::LCtrl, false, &mut commands);
        on_key_down(miyoo, Keycode::LShift, false, &mut commands);
        on_key_down(miyoo, Keycode::LAlt, false, &mut commands);
        on_key_down(miyoo, Keycode::Return, false, &mut commands);
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
        let mut commands = Vec::new();
        on_key_down(
            Keymap::MiyooMini { menu_quits: false },
            Keycode::Escape,
            false,
            &mut commands,
        );
        assert!(commands.is_empty());

        on_key_down(
            Keymap::MiyooMini { menu_quits: true },
            Keycode::Escape,
            false,
            &mut commands,
        );
        assert_eq!(commands, vec![AppCommand::Shutdown]);
    }

    #[test]
    fn arrows_navigate_under_either_layout() {
        for keymap in [Keymap::Desktop, Keymap::MiyooMini { menu_quits: true }] {
            let mut commands = Vec::new();
            on_key_down(keymap, Keycode::Up, false, &mut commands);
            assert_eq!(commands, vec![AppCommand::Nav(Direction::Up)]);
        }
    }
}
