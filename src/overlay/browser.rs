//! Gamepad file browser state machine: directory navigation with a cursor,
//! multi-select across directories, and a root carousel for the handheld's
//! mount points. Pure state — `crate::ui::browser` renders it.

use crate::transfer::files;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Mount points worth offering on handheld CFWs, in preference order.
/// Only the ones that exist become roots; `$HOME` covers the desktop.
const ROOT_CANDIDATES: [&str; 6] = [
    "/roms",
    "/mnt/SDCARD", // the Miyoo card: Onion, Allium, spruce
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
    /// A pinned folder rather than a child of the cwd. Being ordinary rows
    /// keeps the cursor, paging, and `activate` untouched: a pin is just a
    /// directory that happens to sit above the listing.
    pub pinned: bool,
}

/// What one picked row contributes to a send. A folder is walked as it is
/// picked, so the footer can total it before Start.
#[derive(Clone, Copy)]
pub struct Picked {
    pub bytes: u64,
    /// 1 for a file; the tree's file count for a folder.
    pub files: usize,
    pub is_dir: bool,
}

/// What [`FileBrowser::take`] did, for the toast that reports it.
pub enum Taken {
    /// `partial` when the walk left something out.
    Folder {
        name: String,
        files: usize,
        bytes: u64,
        partial: bool,
    },
    /// The files of the folder being looked at.
    Files { files: usize, bytes: u64 },
    /// Given back; `name` is `None` when it was the files of the cwd.
    Given { name: Option<String>, files: usize },
    /// Nothing here to send.
    Nothing,
    /// A picked folder already carries this row.
    Covered { name: String },
}

/// Outcome of a [`FileBrowser::toggle_pin`]: the new list for the config, and
/// which path went in or out (for the toast).
pub struct PinChange {
    pub paths: Vec<String>,
    /// `true` when it was pinned, `false` when it was unpinned.
    pub pinned: bool,
    pub path: PathBuf,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BrowserMode {
    /// Multi-select files to send.
    PickFiles,
    /// Navigate to a directory; Start chooses the cwd.
    PickDir,
}

/// What Start applies the chosen directory to. Only meaningful in
/// [`BrowserMode::PickDir`].
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DirPurpose {
    /// The save-directory setting.
    SaveDir,
    /// The destination folder of the route being added.
    Route,
    /// Where the parked incoming request should land — that request keeps
    /// counting down while this browser is up, so it also owns the input.
    Incoming,
}

pub struct FileBrowser {
    pub open: bool,
    pub mode: BrowserMode,
    pub dir_purpose: DirPurpose,
    /// Shown in the header: who the selection will be sent to, or whose
    /// incoming files are being given a folder.
    pub target_alias: String,
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub cursor: usize,
    /// Picked files and folders, surviving directory navigation. No entry ever
    /// sits inside another: a folder carries its own tree.
    pub selected: BTreeMap<PathBuf, Picked>,
    roots: Vec<PathBuf>,
    root_index: usize,
    /// Pinned folders, shown above every listing so the jump is one press from
    /// wherever the cursor happens to be.
    pinned: Vec<PathBuf>,
}

impl FileBrowser {
    pub fn new() -> Self {
        Self {
            open: false,
            mode: BrowserMode::PickFiles,
            dir_purpose: DirPurpose::SaveDir,
            target_alias: String::new(),
            cwd: PathBuf::new(),
            entries: Vec::new(),
            cursor: 0,
            selected: BTreeMap::new(),
            roots: Vec::new(),
            root_index: 0,
            pinned: Vec::new(),
        }
    }

    /// Open for picking files to send. `extra_roots` and `pinned_paths` come
    /// from the config; `initial` pre-selects paths (the CLI staging list); `start`
    /// is where the last send began, empty on a first run.
    pub fn open_for_send(
        &mut self,
        target_alias: &str,
        extra_roots: &[String],
        pinned_paths: &[String],
        initial: &[PathBuf],
        start: &Path,
    ) {
        self.mode = BrowserMode::PickFiles;
        self.target_alias = target_alias.to_string();
        self.roots = build_roots(extra_roots);
        self.pinned = existing_paths(pinned_paths);
        self.root_index = 0;
        self.selected = initial
            .iter()
            .filter_map(|p| Some((p.clone(), pick_of(p)?)))
            .collect();
        // The CLI can name both a folder and something under it; the folder
        // carries it.
        let dirs: Vec<PathBuf> = self
            .selected
            .iter()
            .filter(|(_, picked)| picked.is_dir)
            .map(|(path, _)| path.clone())
            .collect();
        self.selected
            .retain(|path, _| !dirs.iter().any(|dir| files::is_inside(dir, path)));
        self.cursor = 0;
        self.open = true;
        self.start_at(start);
    }

    /// Open to choose a directory, starting at `start` when it exists.
    /// `purpose` decides what Start does with the folder landed on; `about`
    /// names the peer for the header, empty when the pick is about no one.
    pub fn open_for_dir(
        &mut self,
        start: &Path,
        extra_roots: &[String],
        pinned_paths: &[String],
        purpose: DirPurpose,
        about: &str,
    ) {
        self.mode = BrowserMode::PickDir;
        self.dir_purpose = purpose;
        self.target_alias = about.to_string();
        self.roots = build_roots(extra_roots);
        self.pinned = existing_paths(pinned_paths);
        self.root_index = 0;
        self.selected.clear();
        self.cursor = 0;
        self.open = true;
        self.start_at(start);
    }

    /// Land in `start`, falling back to the first root when it is gone — a
    /// remembered folder can live on a card that is no longer in the slot.
    fn start_at(&mut self, start: &Path) {
        if start.is_dir() && self.change_dir(start.to_path_buf()).is_ok() {
            return;
        }
        if let Some(root) = self.roots.first() {
            let _ = self.change_dir(root.clone());
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.entries.clear();
        self.selected.clear();
    }

    /// (files, total bytes) of the selection; a picked folder counts its tree.
    pub fn selection_totals(&self) -> (usize, u64) {
        self.selected.values().fold((0, 0), |(files, bytes), p| {
            (files + p.files, bytes + p.bytes)
        })
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected.keys().cloned().collect()
    }

    /// The picked folder whose tree already holds `path`, which is then not
    /// pickable on its own.
    pub fn covered_by(&self, path: &Path) -> Option<&Path> {
        self.selected
            .iter()
            .find(|(picked, entry)| entry.is_dir && files::is_inside(picked, path))
            .map(|(picked, _)| picked.as_path())
    }

    /// Whether X takes a folder rather than the files of the cwd.
    pub fn cursor_is_dir(&self) -> bool {
        self.entries.get(self.cursor).is_some_and(|e| e.is_dir)
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

    /// Straight to `index`, for a tapped row. Clamped: the listing can be
    /// rebuilt between the tap and its handling.
    pub fn set_cursor(&mut self, index: usize) {
        self.cursor = index.min(self.entries.len().saturating_sub(1));
    }

    /// A on the cursor row: enter a directory, or toggle a file's selection.
    /// Returns an error message for the toast when the directory is unreadable.
    pub fn activate(&mut self) -> Result<(), String> {
        let Some(entry) = self.entries.get(self.cursor) else {
            return Ok(());
        };
        if entry.is_dir {
            return self.change_dir(entry.path.clone());
        }
        if self.mode != BrowserMode::PickFiles {
            return Ok(()); // PickDir: files aren't selectable
        }
        let (path, size) = (entry.path.clone(), entry.size);
        if self.selected.remove(&path).is_some() {
            return Ok(());
        }
        if let Some(folder) = self.covered_by(&path).map(files::base_name) {
            return Err(format!("Already in {folder}"));
        }
        self.selected.insert(
            path,
            Picked {
                bytes: size,
                files: 1,
                is_dir: false,
            },
        );
        Ok(())
    }

    /// B: go to the parent directory. Returns `false` at a root — the caller
    /// closes the browser.
    pub fn parent(&mut self) -> bool {
        if self.roots.contains(&self.cwd) {
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

    /// X: the folder under the cursor with its whole tree, or — anywhere else —
    /// every file of the folder being looked at. Pressing it again gives back
    /// what it took. `None` while a folder is being chosen.
    pub fn take(&mut self) -> Option<Taken> {
        if self.mode != BrowserMode::PickFiles {
            return None;
        }
        Some(if self.cursor_is_dir() {
            self.take_dir()
        } else {
            self.take_here()
        })
    }

    fn take_dir(&mut self) -> Taken {
        let entry = &self.entries[self.cursor]; // the caller checked the row
        let (path, name) = (entry.path.clone(), entry.name.clone());
        if let Some(given) = self.selected.remove(&path) {
            return Taken::Given {
                name: Some(name),
                files: given.files,
            };
        }
        if let Some(folder) = self.covered_by(&path).map(files::base_name) {
            return Taken::Covered { name: folder };
        }
        let walked = files::walk_folder(&path);
        if walked.files.is_empty() {
            return Taken::Nothing;
        }
        let (files, bytes, partial) = (walked.files.len(), walked.bytes, walked.partial);
        // The folder carries its tree from here on.
        self.selected.retain(|p, _| !files::is_inside(&path, p));
        self.selected.insert(
            path,
            Picked {
                bytes,
                files,
                is_dir: true,
            },
        );
        Taken::Folder {
            name,
            files,
            bytes,
            partial,
        }
    }

    /// Pinned rows are skipped on purpose: they lead the listing but belong to
    /// other folders, and "everything here" must not reach into them.
    fn take_here(&mut self) -> Taken {
        let here: Vec<(PathBuf, u64)> = self
            .entries
            .iter()
            .filter(|e| !e.is_dir && !e.pinned)
            .map(|e| (e.path.clone(), e.size))
            .collect();
        let Some((first, _)) = here.first() else {
            return Taken::Nothing;
        };
        // One answer for the lot: they share the folder they sit in.
        if let Some(folder) = self.covered_by(first).map(files::base_name) {
            return Taken::Covered { name: folder };
        }
        let all_picked = here.iter().all(|(p, _)| self.selected.contains_key(p));
        let mut bytes = 0;
        for (path, size) in &here {
            if all_picked {
                self.selected.remove(path);
            } else {
                self.selected.insert(
                    path.clone(),
                    Picked {
                        bytes: *size,
                        files: 1,
                        is_dir: false,
                    },
                );
            }
            bytes += size;
        }
        if all_picked {
            Taken::Given {
                name: None,
                files: here.len(),
            }
        } else {
            Taken::Files {
                files: here.len(),
                bytes,
            }
        }
    }

    /// How many rows at the top of the listing are pins. They always lead, so
    /// this is also where the folder's own entries begin.
    pub fn pinned_rows(&self) -> usize {
        self.entries.iter().take_while(|e| e.pinned).count()
    }

    /// What Y acts on: the row under the cursor, or the folder being looked at
    /// when there is no row to point at (an empty listing).
    pub fn pin_target(&self) -> Option<PathBuf> {
        if let Some(entry) = self.entries.get(self.cursor) {
            return Some(entry.path.clone());
        }
        (!self.cwd.as_os_str().is_empty()).then(|| self.cwd.clone())
    }

    pub fn target_is_pinned(&self) -> bool {
        self.pin_target().is_some_and(|p| self.pinned.contains(&p))
    }

    /// Y: pin or unpin [`Self::pin_target`]. `None` when there is nothing to
    /// act on at all.
    pub fn toggle_pin(&mut self) -> Option<PinChange> {
        let target = self.pin_target()?;
        let added = match self.pinned.iter().position(|p| *p == target) {
            Some(i) => {
                self.pinned.remove(i);
                false
            }
            None => {
                self.pinned.push(target.clone());
                true
            }
        };
        // The listing carries the pins, so it has to be rebuilt for the row to
        // appear or go away. Pinned rows lead the listing, so the change is
        // always above the cursor: move with it to keep the highlight put.
        let cursor = self.cursor;
        let _ = self.change_dir(self.cwd.clone());
        let shifted = if added {
            cursor + 1
        } else {
            cursor.saturating_sub(1)
        };
        self.cursor = shifted.min(self.entries.len().saturating_sub(1));
        Some(PinChange {
            paths: self
                .pinned
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            pinned: added,
            path: target,
        })
    }

    fn change_dir(&mut self, dir: PathBuf) -> Result<(), String> {
        let entries = read_entries(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        self.cwd = dir;
        self.entries = self.pinned_entries().into_iter().chain(entries).collect();
        self.cursor = 0;
        Ok(())
    }

    /// Pinned rows for the top of the listing. The name shown is the entry's own
    /// name; the renderer puts the full path beside it, since two cards can hold
    /// the same name. Pinned files are dropped in [`BrowserMode::PickDir`] —
    /// there is nothing to do with a file when a folder is being chosen.
    fn pinned_entries(&self) -> Vec<Entry> {
        self.pinned
            .iter()
            .filter_map(|path| {
                let meta = std::fs::metadata(path).ok()?;
                if !meta.is_dir() && self.mode == BrowserMode::PickDir {
                    return None;
                }
                Some(Entry {
                    name: files::base_name(path),
                    path: path.clone(),
                    is_dir: meta.is_dir(),
                    size: if meta.is_dir() { 0 } else { meta.len() },
                    pinned: true,
                })
            })
            .collect()
    }
}

/// The folder's own rows, in the order a folder send walks them.
fn read_entries(dir: &Path) -> std::io::Result<Vec<Entry>> {
    Ok(files::list_dir(dir)?
        .into_iter()
        .map(|e| Entry {
            name: e.name,
            path: e.path,
            is_dir: e.is_dir,
            size: e.size,
            pinned: false,
        })
        .collect())
}

/// A staged path as a selection entry: a file's size, or a folder walked.
/// `None` when it is gone, or a folder with nothing to send.
fn pick_of(path: &Path) -> Option<Picked> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return Some(Picked {
            bytes: meta.len(),
            files: 1,
            is_dir: false,
        });
    }
    let walked = files::walk_folder(path);
    (!walked.files.is_empty()).then_some(Picked {
        bytes: walked.bytes,
        files: walked.files.len(),
        is_dir: true,
    })
}

/// Config paths that exist right now, folders or files, deduplicated, order
/// kept. A pin on a removed SD card is skipped rather than shown as a dead row.
fn existing_paths(paths: &[String]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for path in paths {
        let path = PathBuf::from(path.trim());
        if path.exists() && !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

fn build_roots(extra: &[String]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = ROOT_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect();
    for path in extra
        .iter()
        .cloned()
        .chain(crate::config::env_browser_roots())
    {
        let path = PathBuf::from(path);
        if path.is_dir() && !roots.contains(&path) {
            roots.push(path);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        if home.is_dir() && wants_home_root(&roots, &home) {
            roots.push(home);
        }
    }
    if roots.is_empty() {
        roots.push(PathBuf::from("/"));
    }
    roots
}

/// `$HOME` earns a root of its own only outside every other one: the handheld
/// launchers point it at the app folder, and a root is what B cannot leave.
fn wants_home_root(roots: &[PathBuf], home: &Path) -> bool {
    !roots.iter().any(|root| home.starts_with(root))
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

    /// As the app opens it: roots from the config plus pinned folders.
    fn browser_with_pins(root: &Path, pinned: &[&str]) -> FileBrowser {
        let mut b = FileBrowser::new();
        b.roots = vec![root.to_path_buf()];
        b.pinned = existing_paths(
            &pinned
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>(),
        );
        b.open = true;
        b.change_dir(root.to_path_buf()).unwrap();
        b
    }

    #[test]
    fn pinned_paths_lead_the_listing() {
        let root = temp_tree();
        let games = root.join("games");
        let b = browser_with_pins(&root, &[games.to_str().unwrap()]);

        let names: Vec<&str> = b.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["games", "games", "saves", "readme.txt"]);
        // The first row is the pinned one, the second the real child directory.
        assert!(b.entries[0].pinned);
        assert!(!b.entries[1].pinned);
        // Where the renderer closes the pinned band.
        assert_eq!(b.pinned_rows(), 1);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_pinned_row_navigates_like_a_directory() {
        let root = temp_tree();
        let games = root.join("games");
        let mut b = browser_with_pins(&root, &[games.to_str().unwrap()]);

        b.activate().unwrap();
        assert_eq!(b.cwd, games);
        // And it is still reachable from in there, since it leads the listing.
        assert!(b.entries[0].pinned);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn toggle_pin_acts_on_the_row_under_the_cursor() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        let games = root.join("games");
        assert!(!b.target_is_pinned());

        let change = b.toggle_pin().expect("the cursor is on a row");
        assert!(change.pinned);
        assert_eq!(change.path, games, "the row, not the folder we stand in");
        assert_eq!(change.paths, vec![games.display().to_string()]);
        assert!(b.entries[0].pinned, "the row shows up without a reopen");

        // The cursor followed its row down, so Y now unpins from the real row.
        let change = b.toggle_pin().expect("the cursor is on a row");
        assert!(!change.pinned);
        assert_eq!(change.path, games);
        assert!(change.paths.is_empty());
        assert!(!b.entries[0].pinned, "and goes away again");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_pinned_row_can_be_unpinned_from_the_top_of_the_listing() {
        let root = temp_tree();
        let games = root.join("games");
        let mut b = browser_with_pins(&root, &[games.to_str().unwrap()]);
        assert!(b.target_is_pinned(), "cursor starts on the pinned row");

        let change = b.toggle_pin().expect("the cursor is on a row");
        assert!(!change.pinned);
        assert!(change.paths.is_empty());
        assert!(!b.entries.iter().any(|e| e.pinned));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn files_can_be_pinned_and_keep_their_size() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        b.move_cursor(2); // games, saves, readme.txt

        let change = b.toggle_pin().expect("the cursor is on a row");
        assert!(change.pinned);
        assert_eq!(change.path, root.join("readme.txt"));

        let pinned = &b.entries[0];
        assert!(pinned.pinned && !pinned.is_dir);
        assert_eq!(pinned.size, 2, "\"hi\"");

        // A on it selects the file for sending, as any file row would.
        b.cursor = 0;
        b.activate().unwrap();
        assert!(b.selected.contains_key(&root.join("readme.txt")));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pinned_files_stay_out_of_the_folder_picker() {
        let root = temp_tree();
        let file = root.join("readme.txt");
        let games = root.join("games");
        let mut b = FileBrowser::new();
        b.roots = vec![root.to_path_buf()];
        b.open_for_dir(
            &root,
            &[],
            &[file.display().to_string(), games.display().to_string()],
            DirPurpose::SaveDir,
            "",
        );

        let pinned: Vec<&PathBuf> = b
            .entries
            .iter()
            .filter(|e| e.pinned)
            .map(|e| &e.path)
            .collect();
        assert_eq!(
            pinned,
            vec![&games],
            "a file cannot answer \"choose a folder\""
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_empty_listing_pins_the_folder_being_looked_at() {
        let root = temp_tree();
        let empty = root.join("saves");
        let mut b = browser_at(&root);
        b.change_dir(empty.clone()).unwrap();
        assert!(b.entries.is_empty());

        let change = b
            .toggle_pin()
            .expect("the cwd stands in for the missing row");
        assert!(change.pinned);
        assert_eq!(change.path, empty);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn x_takes_every_file_of_the_folder_and_gives_them_back() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        b.move_cursor(2); // readme.txt; on a directory row X takes the folder

        let Taken::Files { files, bytes } = b.take().expect("files are selectable") else {
            panic!("readme.txt is here");
        };
        assert_eq!(files, 1, "only files; games/ and saves/ are their own rows");
        assert_eq!(bytes, 2);
        assert!(b.selected.contains_key(&root.join("readme.txt")));

        // Pressing it again on a fully selected folder clears it.
        let Taken::Given { name, files } = b.take().expect("files are selectable") else {
            panic!("everything here was already taken");
        };
        assert!(name.is_none(), "the folder itself was never picked");
        assert_eq!(files, 1);
        assert!(b.selected.is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn x_completes_a_partial_selection_instead_of_clearing_it() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        b.change_dir(root.join("games")).unwrap();
        b.activate().unwrap(); // mario.gb, the first row
        assert_eq!(b.selected.len(), 1);

        let Taken::Files { files, .. } = b.take().expect("files are selectable") else {
            panic!("one of two was selected, so X takes the rest");
        };
        assert_eq!(files, 2);
        assert_eq!(b.selected.len(), 2);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn x_leaves_pinned_rows_alone() {
        let root = temp_tree();
        let pinned_file = root.join("games/zelda.gbc");
        let mut b = browser_with_pins(&root, &[pinned_file.to_str().unwrap()]);
        b.set_cursor(3); // the pin, games/, saves/, then readme.txt

        let Taken::Files { files, .. } = b.take().expect("files are selectable") else {
            panic!("readme.txt is here");
        };
        assert_eq!(files, 1, "the pinned row belongs to games/, not here");
        assert!(!b.selected.contains_key(&pinned_file));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn x_takes_the_folder_under_the_cursor_and_gives_it_back() {
        let root = temp_tree();
        let games = root.join("games");
        let mut b = browser_at(&root); // cursor on games/

        let Taken::Folder {
            name,
            files,
            bytes,
            partial,
        } = b.take().expect("files are selectable")
        else {
            panic!("the cursor is on a directory");
        };
        assert_eq!(name, "games");
        assert_eq!((files, bytes), (2, 150));
        assert!(!partial);
        assert_eq!(b.selection_totals(), (2, 150), "the tree, not the row");
        assert!(b.selected[&games].is_dir);

        let Taken::Given { name, files } = b.take().expect("files are selectable") else {
            panic!("it was picked a moment ago");
        };
        assert_eq!((name.as_deref(), files), (Some("games"), 2));
        assert!(b.selected.is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_picked_folder_carries_what_is_under_it() {
        let root = temp_tree();
        let games = root.join("games");
        let mut b = browser_at(&root);
        b.change_dir(games.clone()).unwrap();
        b.activate().unwrap(); // mario.gb
        assert_eq!(b.selection_totals(), (1, 50));

        b.parent();
        b.take().expect("the cursor landed back on games/");
        assert_eq!(
            b.selected.keys().collect::<Vec<_>>(),
            vec![&games],
            "the file it already held went with it"
        );
        assert_eq!(b.selection_totals(), (2, 150));

        // Back inside, its files read as taken and cannot be taken again.
        b.change_dir(games.clone()).unwrap();
        assert_eq!(b.covered_by(&games.join("mario.gb")), Some(games.as_path()));
        assert_eq!(
            b.activate().unwrap_err(),
            "Already in games",
            "A on a file the folder already carries"
        );
        assert_eq!(b.selection_totals(), (2, 150));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn x_reports_a_folder_with_nothing_in_it() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        b.move_cursor(1); // saves/, which is empty

        assert!(matches!(b.take(), Some(Taken::Nothing)));
        assert!(b.selected.is_empty(), "an empty pick would total nothing");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn x_does_nothing_when_a_folder_is_being_chosen() {
        let root = temp_tree();
        let mut b = FileBrowser::new();
        b.roots = vec![root.to_path_buf()];
        b.open_for_dir(&root, &[], &[], DirPurpose::SaveDir, "");
        assert!(b.take().is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_staged_folder_opens_the_browser_already_holding_its_tree() {
        let root = temp_tree();
        let games = root.join("games");
        let mut b = FileBrowser::new();
        b.roots = vec![root.to_path_buf()];
        // The CLI can name both a folder and a file inside it.
        b.open_for_send(
            "Phone",
            &[],
            &[],
            &[games.clone(), games.join("mario.gb")],
            &root,
        );

        assert_eq!(b.selected.keys().collect::<Vec<_>>(), vec![&games]);
        assert_eq!(b.selection_totals(), (2, 150));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pinning_keeps_the_highlight_on_the_same_row() {
        let root = temp_tree();
        let mut b = browser_at(&root);
        b.move_cursor(1); // "saves", with "games" above it
        let before = b.entries[b.cursor].path.clone();

        b.toggle_pin().expect("the cursor is on a row");
        assert_eq!(
            b.entries[b.cursor].path, before,
            "pinned row pushed it down"
        );

        b.toggle_pin().expect("the cursor is on a row");
        assert_eq!(
            b.entries[b.cursor].path, before,
            "and unpinning pulled it back"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pinned_paths_that_no_longer_exist_are_skipped() {
        let root = temp_tree();
        let b = browser_with_pins(
            &root,
            &[
                root.join("games").to_str().unwrap(),
                "/nonexistent/card/roms",
                root.join("readme.txt").to_str().unwrap(),
            ],
        );
        // The folder and the file survive; only the missing card's path is gone.
        let pinned: Vec<&str> = b
            .entries
            .iter()
            .filter(|e| e.pinned)
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(pinned, ["games", "readme.txt"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn start_at_falls_back_when_the_remembered_folder_is_gone() {
        let root = temp_tree();
        let mut b = FileBrowser::new();
        b.roots = vec![root.to_path_buf()];
        b.start_at(Path::new("/nonexistent/card/roms"));
        assert_eq!(b.cwd, root);
        std::fs::remove_dir_all(&root).unwrap();
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
    fn home_inside_a_root_is_not_a_root_of_its_own() {
        let card = PathBuf::from("/mnt/SDCARD");
        let roots = vec![card.clone()];
        assert!(!wants_home_root(&roots, &card.join("Apps/Retsend.pak")));
        assert!(wants_home_root(&roots, Path::new("/home/user")));
        assert!(wants_home_root(&[], Path::new("/home/user")));
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
