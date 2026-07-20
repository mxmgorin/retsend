//! Settings screen state: a cursor over the rows plus the flag that defers
//! the network restart to screen close (port changes shouldn't restart the
//! stack on every step).

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SettingsRow {
    Alias,
    SaveDir,
    Port,
    QuickSave,
    About,
}

const ROWS: [SettingsRow; 5] = [
    SettingsRow::Alias,
    SettingsRow::SaveDir,
    SettingsRow::Port,
    SettingsRow::QuickSave,
    SettingsRow::About,
];

pub const ROW_COUNT: usize = ROWS.len();

pub struct Settings {
    pub open: bool,
    pub cursor: usize,
    /// The port was edited; apply (restart the net stack) on close.
    pub port_dirty: bool,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            open: false,
            cursor: 0,
            port_dirty: false,
        }
    }

    pub fn row(&self) -> SettingsRow {
        ROWS[self.cursor.min(ROW_COUNT - 1)]
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let count = ROW_COUNT as i32;
        self.cursor = (self.cursor as i32 + delta).rem_euclid(count) as usize;
    }
}
