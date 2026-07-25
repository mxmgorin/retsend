//! About screen state: just an open flag. The screen is read-only build
//! metadata (version, commit, date) and the project URL, so there's nothing
//! to navigate — B backs out to Settings.

pub struct AboutView {
    pub open: bool,
}

impl AboutView {
    pub fn new() -> Self {
        Self { open: false }
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}
