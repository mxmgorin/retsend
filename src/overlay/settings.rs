//! Settings screen state: a cursor over the rows plus the flag that defers
//! the network restart to screen close (port changes shouldn't restart the
//! stack on every step).

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SettingsRow {
    Alias,
    SaveDir,
    QuickSave,
    Overwrite,
    KeepFolders,
    AutoRoutes,
    Routes,
    Port,
    About,
}

/// Top-to-bottom order on screen; `crate::ui::settings` labels them in the same
/// order, one cursor indexes both. Port is late — it is set once, if ever.
const ROWS: [SettingsRow; 9] = [
    SettingsRow::Alias,
    SettingsRow::SaveDir,
    SettingsRow::QuickSave,
    SettingsRow::Overwrite,
    SettingsRow::KeepFolders,
    SettingsRow::AutoRoutes,
    SettingsRow::Routes,
    SettingsRow::Port,
    SettingsRow::About,
];

pub const ROW_COUNT: usize = ROWS.len();

pub struct Settings {
    pub cursor: usize,
    /// The port was edited; apply (restart the net stack) when leaving the tab.
    pub port_dirty: bool,
    /// Console folders the save dir currently offers, so the row can say what
    /// "on" actually amounts to. Refreshed on entering the tab and after the
    /// edits that change it; detecting walks the save dir, too much per frame.
    pub auto_route_count: usize,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            port_dirty: false,
            auto_route_count: 0,
        }
    }

    pub fn row(&self) -> SettingsRow {
        ROWS[self.cursor.min(ROW_COUNT - 1)]
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let count = ROW_COUNT as i32;
        self.cursor = (self.cursor as i32 + delta).rem_euclid(count) as usize;
    }

    /// Straight to `index`, for a tapped row.
    pub fn set_cursor(&mut self, index: usize) {
        self.cursor = index.min(ROW_COUNT - 1);
    }
}
