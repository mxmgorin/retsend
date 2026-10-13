//! Settings screen state: a cursor over the rows plus the flag that defers
//! the network restart to screen close (port changes shouldn't restart the
//! stack on every step).

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SettingsRow {
    Alias,
    SaveDir,
    QuickSave,
    Overwrite,
    KeepFolders,
    AutoRoutes,
    Routes,
    Port,
    Scale,
    IdleAnimation,
    SwapAbxy,
    Keys,
    About,
}

impl SettingsRow {
    /// Rows whose value ◂ ▸ walk instead of switching tabs.
    pub fn is_stepper(self) -> bool {
        self == SettingsRow::Scale
    }
}

/// Top-to-bottom order on screen; `crate::ui::settings` labels them in the same
/// order, one cursor indexes both. Port and the scale are late — both are set
/// once, if ever.
const ROWS: [SettingsRow; 13] = [
    SettingsRow::Alias,
    SettingsRow::SaveDir,
    SettingsRow::QuickSave,
    SettingsRow::Overwrite,
    SettingsRow::KeepFolders,
    SettingsRow::AutoRoutes,
    SettingsRow::Routes,
    SettingsRow::Port,
    SettingsRow::Scale,
    SettingsRow::IdleAnimation,
    SettingsRow::SwapAbxy,
    SettingsRow::Keys,
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
    /// The keys editor is offered: not where the pad itself arrives as keys.
    pub keys_row: bool,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            port_dirty: false,
            auto_route_count: 0,
            keys_row: true,
        }
    }

    /// The rows on screen, top to bottom, each with its index in the full
    /// order; the cursor indexes this list.
    pub fn rows(&self) -> Vec<(usize, SettingsRow)> {
        ROWS.into_iter()
            .enumerate()
            .filter(|&(_, row)| self.keys_row || row != SettingsRow::Keys)
            .collect()
    }

    pub fn row(&self) -> SettingsRow {
        let rows = self.rows();
        rows[self.cursor.min(rows.len() - 1)].1
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let count = self.rows().len() as i32;
        self.cursor = (self.cursor as i32 + delta).rem_euclid(count) as usize;
    }

    /// Straight to `index`, for a tapped row.
    pub fn set_cursor(&mut self, index: usize) {
        self.cursor = index.min(self.rows().len() - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hidden_keys_row_is_skipped_by_the_cursor() {
        let mut settings = Settings::new();
        settings.keys_row = false;
        settings.move_cursor(-1);
        assert_eq!(settings.row(), SettingsRow::About);
        settings.move_cursor(-1);
        assert_eq!(settings.row(), SettingsRow::SwapAbxy);
    }
}
