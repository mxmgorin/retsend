//! Destination routing for received files: an optional per-extension folder
//! map layered on top of the default save directory. Built from
//! `[transfer.routes]`, then — unless `auto_routes` is off — from the console
//! folders detected in the save directory; unknown extensions fall back to the
//! default directory. A file the sender named with folders of its own bypasses
//! the routes: a folder transfer arrives as a folder (see `keep_folders`).

use super::files::extension_of;
use crate::net::TransferSettings;
use std::path::{Path, PathBuf};

/// Config keys are hand-editable, so `.GBC` and `gbc` must land on the same
/// route. Shared with the routes editor, which hides an auto route the config
/// already claims.
pub fn normalize_ext(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

/// The distinct directories of `dirs`, in first-use order.
pub fn unique_dirs<'a>(dirs: impl Iterator<Item = &'a Path>) -> Vec<&'a Path> {
    let mut unique: Vec<&Path> = Vec::new();
    for dir in dirs {
        if !unique.contains(&dir) {
            unique.push(dir);
        }
    }
    unique
}

/// One directory as is; several (a session the routes split up) as
/// "first +N more" — for the history rows, which have a single line for it.
pub fn dirs_label<'a>(dirs: impl Iterator<Item = &'a Path>) -> String {
    match unique_dirs(dirs).split_first() {
        None => String::new(),
        Some((first, [])) => first.display().to_string(),
        Some((first, rest)) => format!("{} +{} more", first.display(), rest.len()),
    }
}

/// Resolves each received file to the path it should be saved at.
#[derive(Clone)]
pub struct SaveRouter {
    default_dir: PathBuf,
    /// `(lowercase extension without dot, resolved target dir)`.
    routes: Vec<(String, PathBuf)>,
    /// Replace a file of the same name instead of saving beside it.
    overwrite: bool,
    /// Rebuild the sender's folders under `default_dir` instead of flattening
    /// their files into the extension routes.
    keep_folders: bool,
}

impl SaveRouter {
    /// Build from the save dir the transfer lands in and the `[transfer]`
    /// settings. Extension keys are lowercased (leading dots stripped); a
    /// relative route value resolves under `default_dir`, an absolute one is
    /// kept as is. Blank keys or values are dropped. With `auto_routes`, the
    /// console folders found in `default_dir` fill in the extensions the map
    /// leaves unclaimed.
    pub fn new(default_dir: PathBuf, settings: &TransferSettings) -> Self {
        let auto_routes = settings.auto_routes;
        let mut routes: Vec<(String, PathBuf)> = settings
            .routes
            .iter()
            .filter_map(|(ext, dir)| {
                let ext = normalize_ext(ext);
                let dir = dir.trim();
                if ext.is_empty() || dir.is_empty() {
                    return None;
                }
                let dir = PathBuf::from(dir);
                let dir = if dir.is_absolute() {
                    dir
                } else {
                    default_dir.join(dir)
                };
                Some((ext, dir))
            })
            .collect();
        if auto_routes {
            // Appended, so `dir_for`'s first-match lookup keeps the explicit
            // route whenever both name the same extension.
            for (ext, folder) in crate::config::routes::detect(&default_dir) {
                if !routes.iter().any(|(e, _)| *e == ext) {
                    routes.push((ext, default_dir.join(folder)));
                }
            }
        }
        Self {
            default_dir,
            routes,
            overwrite: settings.overwrite,
            keep_folders: settings.keep_folders,
        }
    }

    /// Everything into `dir`, no extension routes — the folder the user picks
    /// for one request is the whole answer to "where".
    pub fn into_dir(dir: PathBuf, overwrite: bool, keep_folders: bool) -> Self {
        Self {
            default_dir: dir,
            routes: Vec::new(),
            overwrite,
            keep_folders,
        }
    }

    /// Where to save the file at sanitized relative path `rel`: its directory
    /// per [`Self::dir_for`], and a name that clears `taken` (the paths already
    /// given to other files of the same session). See
    /// [`super::files::dest_path`].
    pub fn dest_for(&self, rel: &Path, taken: &std::collections::HashSet<PathBuf>) -> PathBuf {
        super::files::dest_path(&self.dir_for(rel), file_name_of(rel), taken, self.overwrite)
    }

    /// The directory `rel` should land in: the sender's own folders rebuilt
    /// under the save dir, or — for a loose file, or with `keep_folders` off —
    /// its extension's route, falling back to the save dir.
    pub fn dir_for(&self, rel: &Path) -> PathBuf {
        match rel.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(folders) if self.keep_folders => self.default_dir.join(folders),
            _ => self.route_dir(file_name_of(rel)).to_path_buf(),
        }
    }

    /// The fallback directory (for logging / the empty-transfer edge).
    pub fn default_dir(&self) -> &Path {
        &self.default_dir
    }

    /// Where `filenames` would land: one entry per distinct directory, in
    /// first-use order. Names are sanitized exactly as
    /// [`crate::transfer::inbound::InboundSession::new`] does, so the modal
    /// promises the directories the session will actually use — except that a
    /// kept folder reports itself, not every leaf inside it.
    pub fn dest_dirs<'a>(&self, filenames: impl Iterator<Item = &'a str>) -> Vec<String> {
        let dirs: Vec<PathBuf> = filenames
            .map(|name| {
                let rel = super::files::sanitize_relative_path(name);
                self.folder_root(&rel)
                    .unwrap_or_else(|| self.route_dir(file_name_of(&rel)).to_path_buf())
            })
            .collect();
        unique_dirs(dirs.iter().map(PathBuf::as_path))
            .iter()
            .map(|dir| dir.display().to_string())
            .collect()
    }

    /// The extension's route for `filename`, or the default dir when there's
    /// no match.
    fn route_dir(&self, filename: &str) -> &Path {
        let ext = extension_of(filename);
        self.routes
            .iter()
            .find(|(e, _)| *e == ext)
            .map(|(_, dir)| dir.as_path())
            .unwrap_or(&self.default_dir)
    }

    /// The top folder `rel` creates under the save dir, when it has one and we
    /// keep them. `None` leaves the file to the routes.
    fn folder_root(&self, rel: &Path) -> Option<PathBuf> {
        if !self.keep_folders {
            return None;
        }
        let mut components = rel.components();
        let first = components.next()?;
        components.next()?; // a loose file's only component is its own name
        Some(self.default_dir.join(first))
    }
}

/// The file name of a path [`super::files::sanitize_relative_path`] produced.
fn file_name_of(rel: &Path) -> &str {
    rel.file_name()
        .and_then(|n| n.to_str())
        .expect("sanitize_relative_path ends in a non-empty UTF-8 name")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(routes: &[(&str, &str)], auto_routes: bool) -> TransferSettings {
        TransferSettings {
            save_dir: PathBuf::new(),
            auto_accept: false,
            overwrite: false,
            auto_routes,
            keep_folders: true,
            routes: routes
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn router(dir: &str, routes: &[(&str, &str)], auto_routes: bool) -> SaveRouter {
        SaveRouter::new(PathBuf::from(dir), &settings(routes, auto_routes))
    }

    /// The dir a sender-supplied name resolves to, sanitizer included.
    fn dir_of(r: &SaveRouter, sent: &str) -> PathBuf {
        r.dir_for(&super::super::files::sanitize_relative_path(sent))
    }

    #[test]
    fn routes_by_extension_case_insensitively() {
        let r = router("/save", &[("gbc", "gb"), ("PNG", "/shots")], false);
        // Relative route resolves under the default dir.
        assert_eq!(dir_of(&r, "Zelda.gbc"), Path::new("/save/gb"));
        // Key and filename extension are both lowercased; absolute stays put.
        assert_eq!(dir_of(&r, "grab.PNG"), Path::new("/shots"));
        // No route → the default dir; extensionless too.
        assert_eq!(dir_of(&r, "save.dat"), Path::new("/save"));
        assert_eq!(dir_of(&r, "README"), Path::new("/save"));
    }

    #[test]
    fn a_senders_folders_are_rebuilt_and_skip_the_routes() {
        let r = router("/save", &[("gbc", "gb")], false);
        assert_eq!(dir_of(&r, "Zelda/Zelda.gbc"), Path::new("/save/Zelda"));
        assert_eq!(
            dir_of(&r, "Zelda/saves/Zelda.gbc"),
            Path::new("/save/Zelda/saves")
        );
        // Loose files still route.
        assert_eq!(dir_of(&r, "Zelda.gbc"), Path::new("/save/gb"));
    }

    #[test]
    fn keep_folders_off_flattens_into_the_routes() {
        let mut cfg = settings(&[("gbc", "gb")], false);
        cfg.keep_folders = false;
        let r = SaveRouter::new(PathBuf::from("/save"), &cfg);
        assert_eq!(dir_of(&r, "Zelda/saves/Zelda.gbc"), Path::new("/save/gb"));
        assert_eq!(dir_of(&r, "Zelda/notes.txt"), Path::new("/save"));
    }

    #[test]
    fn dirs_label_collapses_duplicates_and_counts_the_rest() {
        let dirs = |paths: &[&str]| dirs_label(paths.iter().map(Path::new));
        assert_eq!(dirs(&[]), "");
        assert_eq!(dirs(&["/save", "/save"]), "/save");
        assert_eq!(dirs(&["/save/gb", "/save", "/save/gb"]), "/save/gb +1 more");
    }

    #[test]
    fn dest_dirs_names_every_dir_a_request_would_use_once() {
        let r = router("/save", &[("png", "shots")], false);
        assert_eq!(r.dest_dirs(["rom.gbc"].into_iter()), ["/save"]);
        assert_eq!(
            r.dest_dirs(["grab.png", "rom.gbc", "shot2.PNG"].into_iter()),
            ["/save/shots", "/save"]
        );
    }

    #[test]
    fn dest_dirs_reports_a_folder_once_however_deep_it_goes() {
        let r = router("/save", &[], false);
        assert_eq!(
            r.dest_dirs(
                [
                    "Zelda/rom.gbc",
                    "Zelda/saves/a.sav",
                    "Zelda/saves/deep/b.sav",
                    "loose.txt",
                ]
                .into_iter()
            ),
            ["/save/Zelda", "/save"]
        );
    }

    #[test]
    fn blank_entries_are_dropped() {
        let r = router("/save", &[("", "x"), ("iso", "")], false);
        assert_eq!(dir_of(&r, "game.iso"), Path::new("/save"));
    }

    /// A temp save dir holding console folders, as a handheld's ROMs root does.
    fn save_dir_with(tag: &str, folders: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "retsend-router-{tag}-{}",
            crate::net::protocol::random_token(4)
        ));
        for folder in folders {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
        }
        dir
    }

    #[test]
    fn auto_routes_fill_unclaimed_extensions() {
        let dir = save_dir_with("auto", &["gba", "snes"]);
        let r = SaveRouter::new(dir.clone(), &settings(&[], true));

        assert_eq!(dir_of(&r, "Zelda.gba"), dir.join("gba"));
        assert_eq!(dir_of(&r, "Mario.sfc"), dir.join("snes"));
        // Nothing detected for it, so the default dir still takes it.
        assert_eq!(dir_of(&r, "notes.txt"), dir);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn explicit_routes_win_over_auto_routes() {
        let dir = save_dir_with("explicit", &["gba"]);
        let r = SaveRouter::new(dir.clone(), &settings(&[("gba", "/elsewhere")], true));
        assert_eq!(dir_of(&r, "Zelda.gba"), Path::new("/elsewhere"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn auto_routes_off_ignores_the_console_folders() {
        let dir = save_dir_with("off", &["gba"]);
        let r = SaveRouter::new(dir.clone(), &settings(&[], false));
        assert_eq!(dir_of(&r, "Zelda.gba"), dir);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn into_dir_takes_everything_but_still_keeps_folders() {
        let r = SaveRouter::into_dir(PathBuf::from("/picked"), false, true);
        assert_eq!(dir_of(&r, "Zelda.gbc"), Path::new("/picked"));
        assert_eq!(
            dir_of(&r, "Zelda/saves/a.sav"),
            Path::new("/picked/Zelda/saves")
        );
    }
}
