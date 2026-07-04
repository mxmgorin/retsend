//! Radar screen state: a cursor over the nearby-devices list. The list itself
//! lives in the peer registry and is snapshotted per frame; only the cursor
//! persists here, clamped against whatever the snapshot returned.

pub struct Home {
    cursor: usize,
}

impl Home {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_clamps_to_list() {
        let mut home = Home::new();
        home.move_cursor(5, 3);
        assert_eq!(home.cursor(3), Some(2));
        // The list shrank (a peer expired): the cursor follows.
        assert_eq!(home.cursor(1), Some(0));
        assert_eq!(home.cursor(0), None);
        home.move_cursor(-10, 3);
        assert_eq!(home.cursor(3), Some(0));
    }
}
