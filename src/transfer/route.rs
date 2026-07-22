//! Destination routing for received files: an optional per-extension folder
//! map layered on top of the default save directory. Built from
//! `[transfer.routes]` (config-file-only, like `browser_roots`); unknown
//! extensions fall back to the default directory.

use super::files::extension_of;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    /// keys or values are dropped.
    pub fn new(default_dir: PathBuf, routes: &BTreeMap<String, String>) -> Self {
        let routes = routes
            .iter()
            .filter_map(|(ext, dir)| {
                let ext = ext.trim().trim_start_matches('.').to_ascii_lowercase();
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
        let r = SaveRouter::new(PathBuf::from("/save"), &map(&[("", "x"), ("iso", "")]));
        assert_eq!(r.dir_for("game.iso"), Path::new("/save"));
    }
}
