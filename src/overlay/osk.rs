//! On-screen keyboard state machine: a character grid navigated with the
//! D-pad, three layers (lower/upper/symbols), and a text buffer. A minimal
//! take on retsurf's OSK — enough for aliases and short strings.

/// What the committed text is for.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum OskTarget {
    Alias,
    /// A new route's file extension; the folder is picked next in the browser.
    RouteExt,
}

/// The user finished (or abandoned) input.
pub enum OskEvent {
    Committed(OskTarget, String),
    Cancelled,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Key {
    Char(char),
    Space,
    Backspace,
    /// Cycle lower → upper → symbols.
    Layer,
    Ok,
}

const CHAR_ROWS: [[&str; 3]; 3] = [
    ["qwertyuiop", "asdfghjkl-", "zxcvbnm._"],
    ["QWERTYUIOP", "ASDFGHJKL-", "ZXCVBNM._"],
    ["1234567890", "!@#$%^&*()", "+=[]{}:;'"],
];
const LAYER_NAMES: [&str; 3] = ["abc", "ABC", "123"];
/// Bottom row: wide special keys.
const SPECIAL_ROW: [Key; 4] = [Key::Layer, Key::Space, Key::Backspace, Key::Ok];

const MAX_LEN: usize = 40;

pub struct Osk {
    pub active: bool,
    pub target: OskTarget,
    pub buffer: String,
    pub row: usize,
    pub col: usize,
    pub layer: usize,
}

impl Osk {
    pub fn new() -> Self {
        Self {
            active: false,
            target: OskTarget::Alias,
            buffer: String::new(),
            row: 0,
            col: 0,
            layer: 0,
        }
    }

    pub fn open(&mut self, target: OskTarget, initial: &str) {
        self.active = true;
        self.target = target;
        self.buffer = initial.to_string();
        self.row = 0;
        self.col = 0;
        self.layer = 0;
    }

    /// Rows of the current layer, the special row last.
    pub fn rows(&self) -> Vec<Vec<Key>> {
        let mut rows: Vec<Vec<Key>> = CHAR_ROWS[self.layer]
            .iter()
            .map(|r| r.chars().map(Key::Char).collect())
            .collect();
        rows.push(SPECIAL_ROW.to_vec());
        rows
    }

    pub fn layer_name(&self) -> &'static str {
        LAYER_NAMES[(self.layer + 1) % LAYER_NAMES.len()]
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let rows = self.rows();
        let row_count = rows.len() as i32;
        self.row = (self.row as i32 + dy).rem_euclid(row_count) as usize;
        let width = rows[self.row].len() as i32;
        // Horizontal wraps within the row; switching rows clamps the column.
        if dx != 0 {
            self.col = (self.col as i32 + dx).rem_euclid(width) as usize;
        } else {
            self.col = self.col.min(width as usize - 1);
        }
    }

    /// A: press the key under the cursor.
    pub fn press(&mut self) -> Option<OskEvent> {
        let rows = self.rows();
        let key = *rows.get(self.row)?.get(self.col)?;
        match key {
            Key::Char(c) => self.push(c),
            Key::Space => self.push(' '),
            Key::Backspace => {
                self.buffer.pop();
            }
            Key::Layer => self.layer = (self.layer + 1) % CHAR_ROWS.len(),
            Key::Ok => return Some(self.commit()),
        }
        None
    }

    /// B: erase; on an empty buffer, cancel out.
    pub fn back(&mut self) -> Option<OskEvent> {
        if self.buffer.is_empty() {
            self.active = false;
            return Some(OskEvent::Cancelled);
        }
        self.buffer.pop();
        None
    }

    /// Start: commit regardless of cursor position.
    pub fn commit(&mut self) -> OskEvent {
        self.active = false;
        OskEvent::Committed(self.target, self.buffer.trim().to_string())
    }

    /// Select: next layer.
    pub fn cycle_layer(&mut self) {
        self.layer = (self.layer + 1) % CHAR_ROWS.len();
    }

    fn push(&mut self, c: char) {
        if self.buffer.chars().count() < MAX_LEN {
            self.buffer.push(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_and_commits() {
        let mut osk = Osk::new();
        osk.open(OskTarget::Alias, "ab");
        osk.press(); // 'q' at 0,0
        assert_eq!(osk.buffer, "abq");
        osk.back();
        assert_eq!(osk.buffer, "ab");
        match osk.commit() {
            OskEvent::Committed(OskTarget::Alias, s) => assert_eq!(s, "ab"),
            _ => panic!("expected commit"),
        }
        assert!(!osk.active);
    }

    #[test]
    fn back_on_empty_cancels() {
        let mut osk = Osk::new();
        osk.open(OskTarget::Alias, "");
        assert!(matches!(osk.back(), Some(OskEvent::Cancelled)));
        assert!(!osk.active);
    }

    #[test]
    fn cursor_wraps_and_clamps() {
        let mut osk = Osk::new();
        osk.open(OskTarget::Alias, "");
        osk.move_cursor(-1, 0); // wrap left on a 10-wide row
        assert_eq!(osk.col, 9);
        osk.move_cursor(0, -1); // up to the 4-wide special row
        assert_eq!(osk.row, 3);
        assert_eq!(osk.col, 3); // clamped
                                // Ok is the last special key.
        assert!(matches!(
            osk.press(),
            Some(OskEvent::Committed(OskTarget::Alias, _))
        ));
    }
}
