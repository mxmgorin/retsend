//! Routes-editor state: a cursor over the configured `ext → folder` routes,
//! a trailing "add" row and the read-only auto routes, plus the extension
//! captured mid-add while the folder is picked in the browser. The routes
//! themselves live in the config; this holds only the editing cursor, the
//! pending extension, and a snapshot of the auto routes to list below the
//! editable ones.

use std::collections::BTreeMap;

/// The single "add route" row between the configured routes and the auto ones.
const ADD_ROWS: usize = 1;

/// Where the cursor sits. Auto routes have nothing to edit; they take cursor
/// positions only so a list longer than the screen can be scrolled into view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteCursor {
    Route(usize),
    Add,
    Auto(usize),
}

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

    /// Cursor over the routes, the "add" row, then the auto routes. Clamped so
    /// a removed route doesn't strand it past the end.
    pub fn cursor(&self, routes: usize, auto: usize) -> RouteCursor {
        match self.cursor.min(Self::last(routes, auto)) {
            c if c < routes => RouteCursor::Route(c),
            c if c == routes => RouteCursor::Add,
            c => RouteCursor::Auto(c - routes - ADD_ROWS),
        }
    }

    pub fn move_cursor(&mut self, delta: i32, routes: usize, auto: usize) {
        let last = Self::last(routes, auto) as i32;
        let current = (self.cursor as i32).min(last);
        self.cursor = (current + delta).clamp(0, last) as usize;
    }

    /// Straight to `index`, for a tapped row.
    pub fn set_cursor(&mut self, index: usize, routes: usize, auto: usize) {
        self.cursor = index.min(Self::last(routes, auto));
    }

    /// The flat position a [`RouteCursor`] sits at — the inverse of
    /// [`Self::cursor`], so a tapped row can name itself without the renderer
    /// knowing where the add row falls.
    pub fn flat_index(cursor: RouteCursor, routes: usize) -> usize {
        match cursor {
            RouteCursor::Route(i) => i,
            RouteCursor::Add => routes,
            RouteCursor::Auto(i) => routes + ADD_ROWS + i,
        }
    }

    fn last(routes: usize, auto: usize) -> usize {
        routes + ADD_ROWS + auto - 1
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
    fn cursor_runs_past_the_add_row_into_the_auto_routes() {
        let mut v = view(&[]);
        assert_eq!(v.cursor(1, 2), RouteCursor::Route(0));
        v.move_cursor(1, 1, 2);
        assert_eq!(v.cursor(1, 2), RouteCursor::Add);
        v.move_cursor(1, 1, 2);
        assert_eq!(v.cursor(1, 2), RouteCursor::Auto(0));
        v.move_cursor(1, 1, 2);
        assert_eq!(v.cursor(1, 2), RouteCursor::Auto(1));
        v.move_cursor(1, 1, 2);
        assert_eq!(
            v.cursor(1, 2),
            RouteCursor::Auto(1),
            "stops at the last row"
        );
    }

    #[test]
    fn cursor_clamps_onto_a_shrunken_list() {
        let mut v = view(&[]);
        v.move_cursor(9, 3, 1);
        // Two routes removed: the stranded cursor lands on the last auto row.
        assert_eq!(v.cursor(1, 1), RouteCursor::Auto(0));
        v.move_cursor(-1, 1, 1);
        assert_eq!(v.cursor(1, 1), RouteCursor::Add);
    }

    #[test]
    fn auto_rows_are_empty_without_detection() {
        let v = view(&[]);
        assert!(v.auto_rows(&configured(&[("gba", "gba")])).is_empty());
    }

    #[test]
    fn flat_index_inverts_the_cursor_mapping() {
        let (routes, auto) = (2, 3);
        let mut view = view(&[("gba", "gba"), ("sfc", "snes"), ("gb", "gb")]);
        for i in 0..routes + 1 + auto {
            view.set_cursor(i, routes, auto);
            let cursor = view.cursor(routes, auto);
            assert_eq!(RoutesView::flat_index(cursor, routes), i, "row {i}");
        }
    }
}
