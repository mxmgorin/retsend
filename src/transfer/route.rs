//! Destination routing for received files: an optional per-extension folder
//! map layered on top of the default save directory. Built from
//! `[transfer.routes]`, then — unless `auto_routes` is off — from the console
//! folders detected in the save directory; unknown extensions fall back to the
//! default directory.

use super::files::extension_of;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Config keys are hand-editable, so `.GBC` and `gbc` must land on the same
/// route. Shared with the routes editor, which hides an auto route the config
/// already claims.
pub fn normalize_ext(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

/// Resolves each received file to the directory it should land in.
#[derive(Clone)]
pub struct SaveRouter {
    default_dir: PathBuf,
    /// `(lowercase extension without dot, resolved target dir)`.
    routes: Vec<(String, PathBuf)>,
}

impl SaveRouter {
    /// Build from the default save dir and the raw `ext -> folder` config map.
    /// Extension keys are lowercased (leading dots stripped); a relative folder
    /// value resolves under `default_dir`, an absolute one is kept as is. Blank
    /// keys or values are dropped. With `auto_routes`, the console folders found
    /// in `default_dir` fill in the extensions the map leaves unclaimed.
    pub fn new(default_dir: PathBuf, routes: &BTreeMap<String, String>, auto_routes: bool) -> Self {
        let mut routes: Vec<(String, PathBuf)> = routes
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
        }
    }

    /// The directory `filename` should land in — its extension's route, or the
    /// default when there's no match.
    pub fn dir_for(&self, filename: &str) -> &Path {
        let ext = extension_of(filename);
        self.routes
            .iter()
            .find(|(e, _)| *e == ext)
            .map(|(_, dir)| dir.as_path())
            .unwrap_or(&self.default_dir)
    }

    /// The fallback directory (for logging / the empty-transfer edge).
    pub fn default_dir(&self) -> &Path {
        &self.default_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn routes_by_extension_case_insensitively() {
        let r = SaveRouter::new(
            PathBuf::from("/save"),
            &map(&[("gbc", "gb"), ("PNG", "/shots")]),
            false,
        );
        // Relative route resolves under the default dir.
        assert_eq!(r.dir_for("Zelda.gbc"), Path::new("/save/gb"));
        // Key and filename extension are both lowercased; absolute stays put.
        assert_eq!(r.dir_for("grab.PNG"), Path::new("/shots"));
        // No route → the default dir; extensionless too.
        assert_eq!(r.dir_for("save.dat"), Path::new("/save"));
        assert_eq!(r.dir_for("README"), Path::new("/save"));
    }

    #[test]
    fn blank_entries_are_dropped() {
        let r = SaveRouter::new(
            PathBuf::from("/save"),
            &map(&[("", "x"), ("iso", "")]),
            false,
        );
        assert_eq!(r.dir_for("game.iso"), Path::new("/save"));
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
        let r = SaveRouter::new(dir.clone(), &Default::default(), true);

        assert_eq!(r.dir_for("Zelda.gba"), dir.join("gba"));
        assert_eq!(r.dir_for("Mario.sfc"), dir.join("snes"));
        // Nothing detected for it, so the default dir still takes it.
        assert_eq!(r.dir_for("notes.txt"), dir);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn explicit_routes_win_over_auto_routes() {
        let dir = save_dir_with("explicit", &["gba"]);
        let r = SaveRouter::new(dir.clone(), &map(&[("gba", "/elsewhere")]), true);
        assert_eq!(r.dir_for("Zelda.gba"), Path::new("/elsewhere"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn auto_routes_off_ignores_the_console_folders() {
        let dir = save_dir_with("off", &["gba"]);
        let r = SaveRouter::new(dir.clone(), &Default::default(), false);
        assert_eq!(r.dir_for("Zelda.gba"), dir);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
