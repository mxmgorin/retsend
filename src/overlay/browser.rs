//! Gamepad file browser state machine: directory navigation with a cursor,
//! multi-select across directories, and a root carousel for the handheld's
//! mount points. Pure state — `crate::ui::browser` renders it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Mount points worth offering on handheld CFWs, in preference order.
/// Only the ones that exist become roots; `$HOME` covers the desktop.
const ROOT_CANDIDATES: [&str; 5] = [
    "/roms",
    "/mnt/mmc",
    "/mnt/sdcard",
    "/userdata/roms",
    "/storage/roms",
];

pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    /// Files only; 0 for directories.
    pub size: u64,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BrowserMode {
    /// Multi-select files to send.
    PickFiles,
    /// Navigate to a directory; Start chooses the cwd (save-dir setting).
    PickDir,
}

pub struct FileBrowser {
    pub open: bool,
    pub mode: BrowserMode,
    /// Shown in the header: who the selection will be sent to.
    pub target_alias: String,
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub cursor: usize,
    /// Selected files (full path → size), surviving directory navigation.
    pub selected: BTreeMap<PathBuf, u64>,
    roots: Vec<PathBuf>,
    root_index: usize,
}

impl FileBrowser {
    pub fn new() -> Self {
        Self {
            open: false,
            mode: BrowserMode::PickFiles,
            target_alias: String::new(),
            cwd: PathBuf::new(),
            entries: Vec::new(),
            cursor: 0,
            selected: BTreeMap::new(),
            roots: Vec::new(),
            root_index: 0,
        }
    }

    /// Open for picking files to send. `extra_roots` come from the config;
    /// `initial` pre-selects files (the CLI staging list).
    pub fn open_for_send(
        &mut self,
        target_alias: &str,
        extra_roots: &[String],
        initial: &[PathBuf],
    ) {
        self.mode = BrowserMode::PickFiles;
        self.target_alias = target_alias.to_string();
        self.roots = build_roots(extra_roots);
        self.root_index = 0;
        self.selected = initial
            .iter()
            .filter_map(|p| Some((p.clone(), std::fs::metadata(p).ok()?.len())))
            .collect();
        self.cursor = 0;
        self.open = true;
        if let Some(root) = self.roots.first() {
            let _ = self.change_dir(root.clone());
        }
    }

    /// Open to choose a directory, starting at `start` when it exists.
    pub fn open_for_dir(&mut self, start: &Path, extra_roots: &[String]) {
        self.mode = BrowserMode::PickDir;
        self.target_alias.clear();
        self.roots = build_roots(extra_roots);
        self.root_index = 0;
        self.selected.clear();
        self.cursor = 0;
        self.open = true;
        let start = if start.is_dir() {
            start.to_path_buf()
        } else {
            self.roots.first().cloned().unwrap_or_else(|| "/".into())
        };
        if self.change_dir(start).is_err() {
            if let Some(root) = self.roots.first() {
                let _ = self.change_dir(root.clone());
            }
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.entries.clear();
        self.selected.clear();
    }

    /// (count, total bytes) of the selection.
    pub fn selection_totals(&self) -> (usize, u64) {
        (self.selected.len(), self.selected.values().sum())
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected.keys().cloned().collect()
    }

    pub fn move_cursor(&mut self, delta: i32) {
        if self.entries.is_empty() {
            self.cursor = 0;
            return;
        }
        let max = self.entries.len() as i32 - 1;
        self.cursor =
            (self.cursor.min(self.entries.len() - 1) as i32 + delta).clamp(0, max) as usize;
    }

    /// A on the cursor row: enter a directory, or toggle a file's selection.
    /// Returns an error message for the toast when the directory is unreadable.
    pub fn activate(&mut self) -> Result<(), String> {
        let Some(entry) = self.entries.get(self.cursor) else {
            return Ok(());
        };
        if entry.is_dir {
            self.change_dir(entry.path.clone())
        } else if self.mode == BrowserMode::PickFiles {
            let path = entry.path.clone();
            let size = entry.size;
            if self.selected.remove(&path).is_none() {
                self.selected.insert(path, size);
            }
            Ok(())
        } else {
            Ok(()) // PickDir: files aren't selectable
        }
    }

    /// B: go to the parent directory. Returns `false` at a root — the caller
    /// closes the browser.
    pub fn parent(&mut self) -> bool {
        if self.roots.iter().any(|r| *r == self.cwd) {
            return false;
        }
        let Some(parent) = self.cwd.parent().map(Path::to_path_buf) else {
            return false;
        };
        let came_from = self.cwd.clone();
        if self.change_dir(parent).is_ok() {
            // Land the cursor on the directory we just left.
            if let Some(i) = self.entries.iter().position(|e| e.path == came_from) {
                self.cursor = i;
            }
        }
        true
    }

    /// Select (the button): jump to the next root mount point.
    pub fn cycle_root(&mut self) -> Option<&Path> {
        if self.roots.is_empty() {
            return None;
        }
        self.root_index = (self.root_index + 1) % self.roots.len();
        let root = self.roots[self.root_index].clone();
        let _ = self.change_dir(root);
        Some(&self.roots[self.root_index])
    }

    fn change_dir(&mut self, dir: PathBuf) -> Result<(), String> {
        let entries = read_entries(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        self.cwd = dir;
        self.entries = entries;
        self.cursor = 0;
        Ok(())
    }
}

/// Directory listing: dirs first, case-insensitive name order, dotfiles
/// hidden, symlinks skipped (a looped symlink tree on an SD card must not
/// hang navigation).
fn read_entries(dir: &Path) -> std::io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if entry.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
            continue;
        }
        entries.push(Entry {
            path: entry.path(),
            is_dir: meta.is_dir(),
            size: if meta.is_dir() { 0 } else { meta.len() },
            name,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

fn build_roots(extra: &[String]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = ROOT_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect();
    for path in extra {
        let path = PathBuf::from(path);
        if path.is_dir() && !roots.contains(&path) {
            roots.push(path);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        if home.is_dir() && !roots.contains(&home) {
            roots.push(home);
        }
    }
    if roots.is_empty() {
        roots.push(PathBuf::from("/"));
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tree() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lsretro-browser-{}",
            crate::net::protocol::random_token(4)
        ));
        std::fs::create_dir_all(dir.join("games")).unwrap();
        std::fs::create_dir_all(dir.join("saves")).unwrap();
        std::fs::write(dir.join("readme.txt"), b"hi").unwrap();
        std::fs::write(dir.join(".hidden"), b"x").unwrap();
        std::fs::write(dir.join("games/zelda.gbc"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("games/mario.gb"), vec![0u8; 50]).unwrap();
        dir
    }

    fn browser_at(root: &Path) -> FileBrowser {
        let mut b = FileBrowser::new();
        b.roots = vec![root.to_path_buf()];
        b.open = true;
        b.change_dir(root.to_path_buf()).unwrap();
        b
    }

    #[test]
    fn lists_dirs_first_and_hides_dotfiles() {
        let root = temp_tree();
        let b = browser_at(&root);
        let names: Vec<&str> = b.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["games", "saves", "readme.txt"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn selection_survives_navigation() {
        let root = temp_tree();
        let mut b = browser_at(&root);

        b.activate().unwrap(); // enter games/
        assert!(b.cwd.ends_with("games"));
        b.move_cursor(1); // mario.gb, zelda.gbc sorted → cursor 0 = mario
        b.activate().unwrap(); // select zelda
        b.move_cursor(-1);
        b.activate().unwrap(); // select mario
        assert_eq!(b.selection_totals(), (2, 150));

        assert!(b.parent()); // back to root, cursor on games/
        assert_eq!(b.entries[b.cursor].name, "games");
        assert_eq!(b.selection_totals(), (2, 150));

        // Toggling off removes from the set.
        b.change_dir(root.join("games")).unwrap();
        b.activate().unwrap(); // deselect mario (cursor 0)
        assert_eq!(b.selection_totals().0, 1);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parent_stops_at_root() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        b.activate().unwrap(); // into games/
        assert!(b.parent()); // back at root
        assert!(!b.parent()); // at root: signal close
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cursor_clamps() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        b.move_cursor(100);
        assert_eq!(b.cursor, b.entries.len() - 1);
        b.move_cursor(-100);
        assert_eq!(b.cursor, 0);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
