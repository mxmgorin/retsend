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
        Keycode::Escape | Keycode::Backspace => AppCommand::Back,
        Keycode::F1 => AppCommand::Start,
        Keycode::Tab | Keycode::F5 => AppCommand::ReAnnounce,
        _ => return,
    };
    commands.push(cmd);
}
