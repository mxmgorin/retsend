//! On-screen keyboard state machine: a character grid navigated with the
//! D-pad, three layers (lower/upper/symbols), and a text buffer. A minimal
//! take on retsurf's OSK — enough for aliases and short strings.

/// What the committed text is for.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum OskTarget {
    Alias,
    /// A new route's file extension; the folder is picked next in the browser.
    RouteExt,
    /// A peer's `ip[:port]`, to add it to the radar by hand.
    PeerAddress,
    /// The port we listen on.
    Port,
}

impl OskTarget {
    /// Layer to open on: an address is digits, dots and colons, which all live
    /// on the symbols layer.
    fn layer(self) -> usize {
        match self {
            OskTarget::Alias | OskTarget::RouteExt => 0,
            OskTarget::PeerAddress | OskTarget::Port => SYMBOL_LAYER,
        }
    }

    /// Label above the buffer — an empty buffer says nothing about what the
    /// keyboard was opened for.
    pub fn prompt(self) -> &'static str {
        match self {
            OskTarget::Alias => "Device name",
            OskTarget::RouteExt => "File extension",
            OskTarget::PeerAddress => "Device IP, e.g. 192.168.1.23",
            OskTarget::Port => "Port, 1024 or above",
        }
    }
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
    // The digits row carries `.`, `:` and the IPv6 brackets below it, so a
    // whole address can be typed without leaving this layer.
    ["1234567890", "!@#$%^&*()", "+=[]{}:;'."],
];
/// Index into [`CHAR_ROWS`] of the digits/punctuation layer.
const SYMBOL_LAYER: usize = 2;
const LAYER_NAMES: [&str; 3] = ["abc", "ABC", "123"];
/// Bottom row: wide special keys.
const SPECIAL_ROW: [Key; 4] = [Key::Layer, Key::Space, Key::Backspace, Key::Ok];

/// `[` + a full IPv6 literal + `]:` + a port — the longest input any target
/// takes.
const MAX_LEN: usize = "[0000:0000:0000:0000:0000:0000:0000:0000]:65535".len();

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
        self.layer = target.layer();
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
    /// X: drop the last character. Leaving the keyboard is B's job, so an empty
    /// buffer simply has nothing to erase.
    pub fn erase(&mut self) {
        self.buffer.pop();
    }

    /// B: leave without committing, whatever has been typed.
    pub fn cancel(&mut self) -> OskEvent {
        self.active = false;
        OskEvent::Cancelled
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
        osk.erase();
        assert_eq!(osk.buffer, "ab");
        match osk.commit() {
            OskEvent::Committed(OskTarget::Alias, s) => assert_eq!(s, "ab"),
            _ => panic!("expected commit"),
        }
        assert!(!osk.active);
    }

    #[test]
    fn erase_on_empty_does_not_cancel() {
        let mut osk = Osk::new();
        osk.open(OskTarget::Alias, "");
        osk.erase();
        assert!(osk.active, "leaving is B's job now, not the erase key's");
        assert_eq!(osk.buffer, "");
    }

    #[test]
    fn cancel_leaves_whatever_was_typed_behind() {
        let mut osk = Osk::new();
        osk.open(OskTarget::Alias, "abc");
        assert!(matches!(osk.cancel(), OskEvent::Cancelled));
        assert!(!osk.active);
    }

    #[test]
    fn an_address_opens_on_the_digits_layer() {
        let mut osk = Osk::new();
        osk.open(OskTarget::PeerAddress, "");
        osk.press(); // '1' at 0,0
        osk.move_cursor(0, 2); // down to the punctuation row
        osk.move_cursor(9, 0); // its last key
        osk.press();
        assert_eq!(osk.buffer, "1.");
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
