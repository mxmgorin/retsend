//! Routes-editor state: a cursor over the configured `ext → folder` routes
//! plus a trailing "add" row, and the extension captured mid-add while the
//! folder is picked in the browser. The routes themselves live in the config;
//! this holds only the editing cursor and the pending extension.

pub struct RoutesView {
    pub open: bool,
    cursor: usize,
    /// Extension typed on the OSK, awaiting a folder pick in the browser. Set
    /// while the add flow is in its second step; cleared when it completes or
    /// the browser is backed out of.
    pub pending_ext: Option<String>,
}

impl RoutesView {
    pub fn new() -> Self {
        Self {
            open: false,
            cursor: 0,
            pending_ext: None,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.cursor = 0;
        self.pending_ext = None;
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
