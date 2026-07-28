//! Routes-editor state: a cursor over the configured `ext → folder` routes
//! plus a trailing "add" row, and the extension captured mid-add while the
//! folder is picked in the browser. The routes themselves live in the config;
//! this holds only the editing cursor, the pending extension, and a snapshot of
//! the auto routes to list below the editable ones.

use std::collections::BTreeMap;

pub struct RoutesView {
    pub open: bool,
    cursor: usize,
    /// Extension typed on the OSK, awaiting a folder pick in the browser. Set
    /// while the add flow is in its second step; cleared when it completes or
    /// the browser is backed out of.
    pub pending_ext: Option<String>,
    /// Auto routes as of the last [`Self::open`] — detecting them walks the save
    /// directory, which is too much for a per-frame rebuild. Empty when the
    /// setting is off.
    auto: BTreeMap<String, String>,
}

impl RoutesView {
    pub fn new() -> Self {
        Self {
            open: false,
            cursor: 0,
            pending_ext: None,
            auto: BTreeMap::new(),
        }
    }

    pub fn open(&mut self, auto: BTreeMap<String, String>) {
        self.open = true;
        self.cursor = 0;
        self.pending_ext = None;
        self.auto = auto;
    }

    /// Auto routes the configured ones don't already claim, as display rows.
    pub fn auto_rows(&self, configured: &BTreeMap<String, String>) -> Vec<(String, String)> {
        let claimed: Vec<String> = configured
            .keys()
            .map(|ext| crate::transfer::route::normalize_ext(ext))
            .collect();
        self.auto
            .iter()
            .filter(|(ext, _)| !claimed.contains(ext))
            .map(|(ext, folder)| (ext.clone(), folder.clone()))
            .collect()
    }

    pub fn close(&mut self) {
        self.open = false;
        self.pending_ext = None;
    }

    /// Cursor over `routes + 1` rows — the last is the "add" row. Clamped so a
    /// removed route doesn't strand it past the end.
    pub fn cursor(&self, routes: usize) -> usize {
        self.cursor.min(routes)
    }

    pub fn move_cursor(&mut self, delta: i32, routes: usize) {
        self.cursor = (self.cursor as i32 + delta).clamp(0, routes as i32) as usize;
    }

    /// The route the cursor is on, or `None` when it's on the add row.
    pub fn selected_route(&self, routes: usize) -> Option<usize> {
        let c = self.cursor(routes);
        (c < routes).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(auto: &[(&str, &str)]) -> RoutesView {
        let mut v = RoutesView::new();
        v.open(
            auto.iter()
                .map(|(e, f)| (e.to_string(), f.to_string()))
                .collect(),
        );
        v
    }

    fn configured(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(e, f)| (e.to_string(), f.to_string()))
            .collect()
    }

    #[test]
    fn auto_rows_drop_extensions_the_config_claims() {
        let v = view(&[("gba", "gba"), ("sfc", "snes")]);
        // A hand-written `.GBA` claims the same extension the router would.
        let rows = v.auto_rows(&configured(&[(".GBA", "/elsewhere")]));
        assert_eq!(rows, vec![("sfc".to_string(), "snes".to_string())]);
    }

    #[test]
    fn auto_rows_are_empty_without_detection() {
        let v = view(&[]);
        assert!(v.auto_rows(&configured(&[("gba", "gba")])).is_empty());
    }
}
