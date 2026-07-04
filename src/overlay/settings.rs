//! Settings screen state. M1: a read-only view of the config (editing — the
//! OSK, the directory picker, the port stepper — lands in the polish
//! milestone); the cursor already navigates so the interaction shape is real.

pub struct Settings {
    pub open: bool,
    pub cursor: usize,
}

/// Rows the renderer shows; the cursor wraps within this count.
pub const ROW_COUNT: usize = 5;

impl Settings {
    pub fn new() -> Self {
        Self {
            open: false,
            cursor: 0,
        }
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let count = ROW_COUNT as i32;
        self.cursor = (self.cursor as i32 + delta).rem_euclid(count) as usize;
    }
}
