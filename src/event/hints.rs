//! Which input the footer hints name: the pad's buttons, or the keyboard's
//! keys. Input handling records what was last used; the renderer asks what a
//! hint's button should read as. Global, like the A/B swap, because every
//! screen's footer draws hints without input state in hand.
//!
//! Per button, a hint shows the key that last fired it, else the first key
//! bound to it. Until a key is pressed the hints stay the pad's.

use crate::config::input::PadButton;
use sdl2::keyboard::Keycode;
use std::sync::Mutex;

const BUTTONS: usize = PadButton::ALL.len();

struct State {
    /// The keyboard, not a pad, was the last input.
    keys: bool,
    /// Per button, indexed by discriminant: its first bound key.
    first: [Option<Keycode>; BUTTONS],
    /// Per button: the key that last fired it, while still bound to it.
    last: [Option<Keycode>; BUTTONS],
}

static STATE: Mutex<State> = Mutex::new(State::new());

impl State {
    const fn new() -> Self {
        Self {
            keys: false,
            first: [None; BUTTONS],
            last: [None; BUTTONS],
        }
    }

    /// Takes the bindings over, forgetting a last key that moved elsewhere.
    fn rebind(
        &mut self,
        first: [Option<Keycode>; BUTTONS],
        owner: impl Fn(Keycode) -> Option<PadButton>,
    ) {
        self.first = first;
        for button in PadButton::ALL {
            let slot = &mut self.last[button as usize];
            if slot.is_some_and(|kc| owner(kc) != Some(button)) {
                *slot = None;
            }
        }
    }

    fn key_used(&mut self, button: PadButton, kc: Keycode) {
        // Android's Back is also the touch screen's gesture: no keyboard implied.
        if kc != Keycode::AC_BACK {
            self.keys = true;
        }
        self.last[button as usize] = Some(kc);
    }

    fn key_for(&self, button: PadButton) -> Option<Keycode> {
        self.last[button as usize].or(self.first[button as usize])
    }

    /// What `button` reads as while the keyboard leads; `None` keeps the pad's.
    fn label(&self, button: &str) -> Option<String> {
        if !self.keys {
            return None;
        }
        let key = |pad| self.key_for(pad).map(hint_label);
        match button {
            "A" => key(PadButton::A),
            "B" => key(PadButton::B),
            "X" => key(PadButton::X),
            "Y" => key(PadButton::Y),
            "Start" => key(PadButton::Start),
            "Select" => key(PadButton::Select),
            "L1 R1" => Some(format!("{} {}", key(PadButton::L1)?, key(PadButton::R1)?)),
            _ => None,
        }
    }
}

/// Publish the keymap's bindings: each button's first key, and which button
/// a key now belongs to.
pub fn rebind(first: [Option<Keycode>; BUTTONS], owner: impl Fn(Keycode) -> Option<PadButton>) {
    lock().rebind(first, owner);
}

pub fn key_used(button: PadButton, kc: Keycode) {
    lock().key_used(button, kc);
}

pub fn pad_used() {
    lock().keys = false;
}

/// The key a hint should name for `button`, or `None` to draw the pad's.
pub fn label(button: &str) -> Option<String> {
    lock().label(button)
}

fn lock() -> std::sync::MutexGuard<'static, State> {
    // Every write is a plain field store, so a poisoned state is still whole.
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// A key's screen label, shortened to fit a cap.
fn hint_label(kc: Keycode) -> String {
    let name = kc.name();
    match super::key_label(&name) {
        "Backspace" => "Bksp".to_string(),
        "Num Enter" => "Enter".to_string(),
        "Left soft key" => "L soft".to_string(),
        "Right soft key" => "R soft".to_string(),
        "L Shift" | "R Shift" => "Shift".to_string(),
        "L Alt" | "R Alt" => "Alt".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> Keycode {
        Keycode::from_name(name).expect("test key names are SDL's")
    }

    fn bound() -> State {
        let mut state = State::new();
        let mut first = [None; BUTTONS];
        first[PadButton::A as usize] = Some(key("Return"));
        first[PadButton::L1 as usize] = Some(key("PageUp"));
        first[PadButton::R1 as usize] = Some(key("PageDown"));
        state.rebind(first, |_| None);
        state
    }

    #[test]
    fn button_discriminants_index_all() {
        for (i, button) in PadButton::ALL.into_iter().enumerate() {
            assert_eq!(button as usize, i);
        }
    }

    #[test]
    fn hints_stay_the_pads_until_a_key_is_pressed() {
        let mut state = bound();
        assert_eq!(state.label("A"), None);
        state.key_used(PadButton::B, key("Escape"));
        assert_eq!(state.label("A").as_deref(), Some("Enter"));
        assert_eq!(state.label("B").as_deref(), Some("Esc"));
        assert_eq!(state.label("L1 R1").as_deref(), Some("PgUp PgDn"));
    }

    #[test]
    fn the_last_key_used_wins_over_the_first_bound() {
        let mut state = bound();
        state.key_used(PadButton::A, key("Select"));
        assert_eq!(state.label("A").as_deref(), Some("OK"));
    }

    #[test]
    fn the_pad_takes_the_hints_back() {
        let mut state = bound();
        state.key_used(PadButton::A, key("Return"));
        state.keys = false;
        assert_eq!(state.label("A"), None);
    }

    #[test]
    fn a_last_key_moved_to_another_button_is_forgotten() {
        let mut state = bound();
        state.key_used(PadButton::A, key("5"));
        let first = state.first;
        state.rebind(first, |kc| (kc == key("5")).then_some(PadButton::Start));
        assert_eq!(state.label("A").as_deref(), Some("Enter"));
    }

    #[test]
    fn an_unbound_button_keeps_the_pads_hint() {
        let mut state = bound();
        state.key_used(PadButton::A, key("Return"));
        assert_eq!(state.label("Y"), None);
    }

    #[test]
    fn androids_back_gesture_does_not_switch_to_keys() {
        let mut state = bound();
        state.key_used(PadButton::B, Keycode::AC_BACK);
        assert_eq!(state.label("A"), None);
        state.key_used(PadButton::A, key("Select"));
        assert_eq!(state.label("B").as_deref(), Some("Back"));
    }

    #[test]
    fn either_shift_or_alt_reads_as_one_cap() {
        let mut state = bound();
        state.key_used(PadButton::Select, Keycode::RShift);
        state.key_used(PadButton::Start, Keycode::LAlt);
        assert_eq!(state.label("Select").as_deref(), Some("Shift"));
        assert_eq!(state.label("Start").as_deref(), Some("Alt"));
    }
}
