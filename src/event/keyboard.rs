//! Desktop-dev keyboard mirror of the gamepad controls. The handheld has no
//! keyboard; this exists so every flow is drivable while developing.

use crate::app::{AppCommand, Direction};
use sdl2::keyboard::Keycode;

pub fn on_key_down(kc: Keycode, repeat: bool, commands: &mut Vec<AppCommand>) {
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
        Keycode::Return | Keycode::KpEnter => AppCommand::Confirm,
        Keycode::Escape => AppCommand::Back,
        // Both the pad's label and the key a desktop hand reaches for.
        Keycode::X | Keycode::Backspace => AppCommand::Erase,
        Keycode::F1 => AppCommand::Start,
        Keycode::Tab | Keycode::F5 => AppCommand::ReAnnounce,
        Keycode::Y => AppCommand::TogglePin,
        _ => return,
    };
    commands.push(cmd);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_and_backspace_erase_while_escape_backs_out() {
        let mut commands = Vec::new();
        on_key_down(Keycode::X, false, &mut commands);
        on_key_down(Keycode::Backspace, false, &mut commands);
        on_key_down(Keycode::Escape, false, &mut commands);
        assert_eq!(
            commands,
            vec![AppCommand::Erase, AppCommand::Erase, AppCommand::Back]
        );
    }

    #[test]
    fn y_pins_and_does_not_repeat() {
        let mut commands = Vec::new();
        on_key_down(Keycode::Y, false, &mut commands);
        assert_eq!(commands, vec![AppCommand::TogglePin]);

        // Holding it must not toggle over and over.
        commands.clear();
        on_key_down(Keycode::Y, true, &mut commands);
        assert!(commands.is_empty());
    }
}
