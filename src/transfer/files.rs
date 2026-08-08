//! Filename hygiene for received files. Senders control `fileName` byte for
//! byte, so this is a security boundary: strip control characters and
//! FAT-illegal characters (handheld SD cards are FAT), keep every component a
//! name of its own, and never let a path escape the save directory.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Longest allowed name in bytes — comfortably under every filesystem's 255
/// while leaving room for the ` (N)` collision suffix and `.part`.
const MAX_NAME_BYTES: usize = 200;
/// Of which the extension may take at most this, leaving the stem a budget.
const MAX_EXT_BYTES: usize = 20;
/// Deepest folder chain rebuilt from a sender's path; the rest collapse onto
/// it, so no peer can nest a transfer down to the filesystem's path limit.
const MAX_DEPTH: usize = 8;
/// Stands in for a component that sanitizes away to nothing.
const FALLBACK_NAME: &str = "file";

/// Reduce an untrusted sender-supplied file name to a safe basename, dropping
/// any directory components. Guarantees a non-empty result with no separators,
/// no control characters, no FAT-illegal characters, and no leading/trailing
/// dots or spaces (so `.` and `..` are impossible).
pub fn sanitize_filename(raw: &str) -> String {
    // Last path component only: both separator styles, plus NUL just in case.
    let last = raw.rsplit(['/', '\\', '\0']).next().unwrap_or_default();
    clean_component(last).unwrap_or_else(|| FALLBACK_NAME.to_string())
}

/// Reduce a sender-supplied name — which protocol v2 lets carry directory
/// components, that being how folder transfers travel — to a safe relative
/// path. Components sanitize as in [`sanitize_filename`], and ones with no
/// name of their own (`.`, `..`, empty) drop out, so joining the result onto
/// the save directory can never leave it.
pub fn sanitize_relative_path(raw: &str) -> PathBuf {
    let mut components = raw.split(['/', '\\', '\0']);
    // `split` yields at least one item, and the file name is the last of them.
    let name = components
        .next_back()
        .and_then(clean_component)
        .unwrap_or_else(|| FALLBACK_NAME.to_string());
    let mut path: PathBuf = components
        .filter_map(clean_component)
        .take(MAX_DEPTH)
        .collect();
    path.push(name);
    path
}

/// One path component reduced to a legal name, or `None` when nothing of it
/// survives. Splitting is the caller's job — separators never reach here.
fn clean_component(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            c if c.is_control() => '_',
            ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect();

    // Leading dots would make dotfiles (or `.`/`..`); trailing dots/spaces
    // are invalid on FAT and invisible everywhere else.
    let trimmed = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.len() <= MAX_NAME_BYTES {
        return Some(trimmed.to_string());
    }
    // Over-long: keep the extension (it routes files on the device) and
    // truncate the stem on a char boundary.
    let (stem, ext) = split_extension(trimmed);
    let ext = truncate_chars(ext, MAX_EXT_BYTES);
    let stem = truncate_chars(stem, MAX_NAME_BYTES - ext.len());
    Some(format!("{stem}{ext}"))
}

/// The path in `dir` to save `name` at. A collision steps the name aside —
/// `name (1).gbc`, `name (2).gbc`, … — unless `overwrite`, which replaces the
/// file on disk. `taken` (other files of this session) steps aside either way.
pub fn dest_path(dir: &Path, name: &str, taken: &HashSet<PathBuf>, overwrite: bool) -> PathBuf {
    // A directory can't be renamed onto, so it collides in either mode.
    let free =
        |p: &PathBuf| !taken.contains(p) && if overwrite { !p.is_dir() } else { !p.exists() };
    let candidate = dir.join(name);
    if free(&candidate) {
        return candidate;
    }
    let (stem, ext) = split_extension(name);
    for i in 1u32.. {
        let candidate = dir.join(format!("{stem} ({i}){ext}"));
        if free(&candidate) {
            return candidate;
        }
    }
    unreachable!("u32 exhausted searching for a free name");
}

/// MIME type by extension for outbound file metadata. Receivers use it only
/// to pick an icon (and previews for images), so a small table plus the
/// octet-stream default covers everything a handheld sends.
pub fn mime_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("txt" | "md" | "log" | "cfg" | "ini") => "text/plain",
        Some("json") => "application/json",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("7z") => "application/x-7z-compressed",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("mp4") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        // ROMs, saves, and everything else.
        _ => "application/octet-stream",
    }
}

/// Remove leftover `.part` files older than a day from `dir` — debris from
/// crashes or yanked power mid-transfer. Fresh ones are left alone in case a
/// transfer is somehow still running. Called once at startup, best-effort.
pub fn sweep_stale_parts(dir: &Path) {
    const MAX_AGE: std::time::Duration = std::time::Duration::from_secs(24 * 3600);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_part = path.extension().is_some_and(|e| e == "part");
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > MAX_AGE);
        if is_part && stale {
            match std::fs::remove_file(&path) {
                Ok(()) => log::info!("swept stale `{}`", path.display()),
                Err(e) => log::warn!("could not sweep `{}`: {e}", path.display()),
            }
        }
    }
}

/// Sibling `.part` path the file streams into before the final rename.
pub fn part_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".part");
    PathBuf::from(os)
}

/// `("archive.tar", ".gz")`-style split on the last dot; names without a dot
/// (or with only a leading one — impossible after sanitize) get an empty ext.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => name.split_at(i),
        _ => (name, ""),
    }
}

/// The lowercase extension without the dot (`"foo.GBC"` → `"gbc"`), or `""`
/// when there's none. Used to route received files to per-extension folders.
pub fn extension_of(name: &str) -> String {
    let (_, ext) = split_extension(name);
    ext.trim_start_matches('.').to_ascii_lowercase()
}

fn truncate_chars(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_paths_and_traversal() {
        for (hostile, expected) in [
            ("../../etc/passwd", "passwd"),
            ("/etc/passwd", "passwd"),
            ("..\\..\\windows\\system32\\cfg", "cfg"),
            ("a/b/c.gbc", "c.gbc"),
            ("..", "file"),
            (".", "file"),
            ("", "file"),
            ("...", "file"),
            (".hidden", "hidden"),
            ("name.", "name"),
            (" spaced ", "spaced"),
        ] {
            assert_eq!(sanitize_filename(hostile), expected, "input `{hostile}`");
        }
    }

    #[test]
    fn sanitize_replaces_illegal_characters() {
        assert_eq!(sanitize_filename("a:b*c?d\"e<f>g|h"), "a_b_c_d_e_f_g_h");
        // NUL acts as a separator (defense against truncation smuggling):
        // only what follows it survives; other control chars become `_`.
        assert_eq!(sanitize_filename("nul\0byte\ntab\t.gbc"), "byte_tab_.gbc");
    }

    #[test]
    fn sanitize_caps_length_keeping_extension() {
        let long = format!("{}.gbc", "x".repeat(300));
        let out = sanitize_filename(&long);
        assert!(out.len() <= 200, "len {}", out.len());
        assert!(out.ends_with(".gbc"));

        // Multi-byte chars must not be split mid-boundary.
        let cyrillic = format!("{}.sav", "ы".repeat(300));
        let out = sanitize_filename(&cyrillic);
        assert!(out.len() <= 200);
        assert!(out.ends_with(".sav"));
    }

    #[test]
    fn sanitized_name_stays_inside_save_dir() {
        let dir = Path::new("/tmp/save");
        for hostile in ["../../etc/passwd", "a/../../b", "..\\..\\x", "\0/etc/x"] {
            let joined = dir.join(sanitize_filename(hostile));
            assert_eq!(joined.parent(), Some(dir), "input `{hostile}`");
        }
    }

    #[test]
    fn relative_path_keeps_the_senders_folders() {
        for (sent, expected) in [
            ("Roms/gb/Zelda.gbc", "Roms/gb/Zelda.gbc"),
            // Windows senders use backslashes.
            ("Roms\\gb\\Zelda.gbc", "Roms/gb/Zelda.gbc"),
            ("Zelda.gbc", "Zelda.gbc"),
            // Empty and `.` components drop out.
            ("Roms//gb/./Zelda.gbc", "Roms/gb/Zelda.gbc"),
            // Illegal characters are replaced per component.
            ("R:oms/g*b/Ze?lda.gbc", "R_oms/g_b/Ze_lda.gbc"),
        ] {
            assert_eq!(
                sanitize_relative_path(sent),
                PathBuf::from(expected),
                "input `{sent}`"
            );
        }
    }

    #[test]
    fn relative_path_cannot_escape_the_save_dir() {
        let dir = Path::new("/tmp/save");
        for hostile in [
            "../../etc/passwd",
            "roms/../../../etc/passwd",
            "..\\..\\windows\\system32\\cfg",
            "/etc/passwd",
            "\0/etc/passwd",
            "roms/..",
            "..",
            "",
        ] {
            let joined = dir.join(sanitize_relative_path(hostile));
            assert!(joined.starts_with(dir), "input `{hostile}` → {joined:?}");
            assert!(
                !joined.components().any(|c| c.as_os_str() == ".."),
                "input `{hostile}` kept a `..`"
            );
        }
        // The traversal is gone, the name it pointed at survives.
        assert_eq!(
            sanitize_relative_path("roms/../../../etc/passwd"),
            PathBuf::from("roms/etc/passwd")
        );
    }

    #[test]
    fn relative_path_caps_depth_and_component_length() {
        let deep: String = (0..MAX_DEPTH + 5)
            .map(|i| format!("d{i}/"))
            .collect::<String>()
            + "game.gbc";
        let path = sanitize_relative_path(&deep);
        assert_eq!(path.components().count(), MAX_DEPTH + 1, "{path:?}");
        assert_eq!(path.file_name().unwrap(), "game.gbc");
        // The leading folders are the ones kept.
        assert!(path.starts_with("d0/d1"), "{path:?}");

        let long = format!("{}/{}.gbc", "x".repeat(300), "y".repeat(300));
        for component in sanitize_relative_path(&long).components() {
            assert!(component.as_os_str().len() <= MAX_NAME_BYTES);
        }
    }

    #[test]
    fn relative_path_always_ends_in_a_name() {
        // A trailing separator leaves no name to use.
        assert_eq!(
            sanitize_relative_path("roms/gb/"),
            PathBuf::from("roms/gb").join(FALLBACK_NAME)
        );
        assert_eq!(sanitize_relative_path(""), PathBuf::from(FALLBACK_NAME));
    }

    #[test]
    fn dest_path_suffixes_collisions() {
        let dir = std::env::temp_dir().join(format!("lsretro-files-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut taken = HashSet::new();

        let first = dest_path(&dir, "game.gbc", &taken, false);
        assert_eq!(first, dir.join("game.gbc"));
        taken.insert(first);

        // Second file of the same session with the same name.
        let second = dest_path(&dir, "game.gbc", &taken, false);
        assert_eq!(second, dir.join("game (1).gbc"));
        taken.insert(second);

        // A name already on disk collides too.
        std::fs::write(dir.join("save.dat"), b"x").unwrap();
        let third = dest_path(&dir, "save.dat", &taken, false);
        assert_eq!(third, dir.join("save (1).dat"));

        // No extension.
        taken.insert(dir.join("README"));
        let fourth = dest_path(&dir, "README", &taken, false);
        assert_eq!(fourth, dir.join("README (1)"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn dest_path_overwrites_disk_but_not_the_session() {
        let dir = std::env::temp_dir().join(format!("lsretro-files-ow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("game.gbc"), b"old").unwrap();
        std::fs::create_dir_all(dir.join("folder")).unwrap();
        let mut taken = HashSet::new();

        let first = dest_path(&dir, "game.gbc", &taken, true);
        assert_eq!(first, dir.join("game.gbc"));
        taken.insert(first);

        // Same name twice in one transfer still needs two paths.
        let second = dest_path(&dir, "game.gbc", &taken, true);
        assert_eq!(second, dir.join("game (1).gbc"));

        // Renaming onto a directory would fail, so it steps aside.
        assert_eq!(
            dest_path(&dir, "folder", &taken, true),
            dir.join("folder (1)")
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn part_path_appends_suffix() {
        assert_eq!(
            part_path(Path::new("/save/game.gbc")),
            PathBuf::from("/save/game.gbc.part")
        );
    }
}
