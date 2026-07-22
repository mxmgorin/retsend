//! History screen state: a cursor over the transfer log. The log itself lives
//! in `App` (persisted); only the cursor persists here, clamped against the
//! entry count each frame — the same shape as [`super::home::Home`].

pub struct HistoryView {
    cursor: usize,
}

impl HistoryView {
    pub fn new() -> Self {
        Self { cursor: 0 }
    }

    /// Cursor position, valid for a list of `len` items (`None` when empty).
    pub fn cursor(&self, len: usize) -> Option<usize> {
        (len > 0).then(|| self.cursor.min(len - 1))
    }

    pub fn move_cursor(&mut self, delta: i32, len: usize) {
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let cur = self.cursor.min(len - 1) as i32;
        self.cursor = (cur + delta).clamp(0, len as i32 - 1) as usize;
    }
}
