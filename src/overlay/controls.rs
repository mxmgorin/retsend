//! Keys-editor state: a cursor over the buttons and the "remap all" and
//! "reset all" rows, and what the next key press is for. The bindings live in
//! the config.

use crate::config::input::PadButton;
use crate::event::Layout;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

/// How long a capture waits for a key: a pad alone can never answer it.
pub const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);

/// The "remap all" and "reset all" rows after the buttons.
const TAIL_ROWS: usize = 2;
const ROW_COUNT: usize = PadButton::ALL.len() + TAIL_ROWS;

/// The buttons a remap asks for, in order. Menu is the launcher's: not asked.
pub const REMAP_ORDER: [PadButton; 12] = [
    PadButton::Up,
    PadButton::Down,
    PadButton::Left,
    PadButton::Right,
    PadButton::A,
    PadButton::B,
    PadButton::X,
    PadButton::Y,
    PadButton::Start,
    PadButton::Select,
    PadButton::L1,
    PadButton::R1,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsRow {
    Button(PadButton),
    RemapAll,
    ResetAll,
}

/// What the next key press is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    Off,
    /// A key to add to the row under the cursor.
    Row,
    /// A remap's next key; holds those taken so far, in [`REMAP_ORDER`].
    Remap(Vec<Keycode>),
}

/// What a key did to a running remap.
#[derive(Debug, PartialEq, Eq)]
pub enum RemapStep {
    /// Already given to this button earlier in the remap; asked again.
    Taken(PadButton),
    Next,
    /// Every button has its key, in [`REMAP_ORDER`].
    Done(Vec<(PadButton, Keycode)>),
}

pub struct ControlsView {
    pub open: bool,
    cursor: usize,
    capture: Capture,
    /// When the wait for the current key began.
    armed_at: Option<Instant>,
    /// The preset the overrides land on, for the keys a button keeps.
    pub layout: Layout,
}

impl ControlsView {
    pub fn new() -> Self {
        Self {
            open: false,
            cursor: 0,
            capture: Capture::Off,
            armed_at: None,
            layout: Layout::default(),
        }
    }

    pub fn open(&mut self, layout: Layout) {
        self.open = true;
        self.layout = layout;
        self.cursor = 0;
        self.disarm();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.disarm();
    }

    pub fn capture(&self) -> &Capture {
        &self.capture
    }

    /// Wait for a key for `capture`, the clock starting now.
    pub fn arm(&mut self, capture: Capture) {
        self.capture = capture;
        self.armed_at = Some(Instant::now());
    }

    pub fn disarm(&mut self) {
        self.capture = Capture::Off;
        self.armed_at = None;
    }

    pub fn is_capturing(&self) -> bool {
        self.capture != Capture::Off
    }

    pub fn timed_out(&self) -> bool {
        self.time_left().is_some_and(|t| t.is_zero())
    }

    /// Until the wait gives up, while one runs.
    pub fn time_left(&self) -> Option<Duration> {
        let armed_at = self.armed_at?;
        Some(CAPTURE_TIMEOUT.saturating_sub(armed_at.elapsed()))
    }

    /// The button a remap is asking for, while one runs.
    pub fn remap_button(&self) -> Option<PadButton> {
        match &self.capture {
            Capture::Remap(keys) => REMAP_ORDER.get(keys.len()).copied(),
            _ => None,
        }
    }

    /// Give the remap's current button `kc`. Ends the remap once done.
    pub fn remap_key(&mut self, kc: Keycode) -> RemapStep {
        let Capture::Remap(keys) = &mut self.capture else {
            unreachable!("remap_key is only called while a remap runs");
        };
        if let Some(at) = keys.iter().position(|&k| k == kc) {
            self.armed_at = Some(Instant::now());
            return RemapStep::Taken(REMAP_ORDER[at]);
        }
        keys.push(kc);
        if keys.len() < REMAP_ORDER.len() {
            self.armed_at = Some(Instant::now());
            return RemapStep::Next;
        }
        let done = REMAP_ORDER.into_iter().zip(keys.iter().copied()).collect();
        self.disarm();
        RemapStep::Done(done)
    }

    pub fn row(&self) -> ControlsRow {
        let buttons = PadButton::ALL.len();
        match self.cursor {
            i if i < buttons => ControlsRow::Button(PadButton::ALL[i]),
            i if i == buttons => ControlsRow::RemapAll,
            _ => ControlsRow::ResetAll,
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let last = (ROW_COUNT - 1) as i32;
        self.cursor = (self.cursor as i32 + delta).clamp(0, last) as usize;
    }

    /// Straight to `index`, for a tapped row.
    pub fn set_cursor(&mut self, index: usize) {
        self.cursor = index.min(ROW_COUNT - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_runs_over_the_buttons_onto_remap_and_reset_and_stops() {
        let mut view = ControlsView::new();
        assert_eq!(view.row(), ControlsRow::Button(PadButton::ALL[0]));
        view.move_cursor(ROW_COUNT as i32 + 5);
        assert_eq!(view.row(), ControlsRow::ResetAll);
        view.move_cursor(-1);
        assert_eq!(view.row(), ControlsRow::RemapAll);
        view.move_cursor(-1);
        assert_eq!(view.row(), ControlsRow::Button(PadButton::Menu));
        view.move_cursor(-100);
        assert_eq!(view.cursor(), 0);
    }

    #[test]
    fn a_remap_asks_every_button_once_and_refuses_a_repeat() {
        let mut view = ControlsView::new();
        view.arm(Capture::Remap(Vec::new()));
        let keys: Vec<Keycode> = (0..REMAP_ORDER.len())
            .map(|i| Keycode::from_i32(Keycode::A.into_i32() + i as i32).expect("a letter"))
            .collect();

        assert_eq!(view.remap_button(), Some(PadButton::Up));
        assert_eq!(view.remap_key(keys[0]), RemapStep::Next);
        assert_eq!(view.remap_key(keys[0]), RemapStep::Taken(PadButton::Up));
        assert_eq!(view.remap_button(), Some(PadButton::Down));

        for &kc in &keys[1..keys.len() - 1] {
            assert_eq!(view.remap_key(kc), RemapStep::Next);
        }
        let RemapStep::Done(done) = view.remap_key(keys[keys.len() - 1]) else {
            panic!("the last key ends the remap");
        };
        assert_eq!(done.len(), REMAP_ORDER.len());
        assert_eq!(done[4], (PadButton::A, keys[4]));
        assert!(!view.is_capturing());
        assert_eq!(view.time_left(), None);
    }

    #[test]
    fn an_armed_wait_counts_down_from_the_timeout() {
        let mut view = ControlsView::new();
        assert_eq!(view.time_left(), None);
        view.arm(Capture::Row);
        let left = view.time_left().expect("armed");
        assert!(left <= CAPTURE_TIMEOUT && left > CAPTURE_TIMEOUT / 2);
    }
}
