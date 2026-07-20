//! Filename hygiene for received files. Senders control `fileName` byte for
//! byte, so this is a security boundary: strip path components, control
//! characters, and FAT-illegal characters (handheld SD cards are FAT), and
//! never let a name escape the save directory.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Longest allowed name in bytes — comfortably under every filesystem's 255
/// while leaving room for the ` (N)` uniquing suffix and `.part`.
const MAX_NAME_BYTES: usize = 200;

/// Reduce an untrusted sender-supplied file name to a safe basename.
/// Guarantees a non-empty result with no separators, no control characters,
/// no FAT-illegal characters, and no leading/trailing dots or spaces (so `.`
/// and `..` are impossible).
pub fn sanitize_filename(raw: &str) -> String {
    // Last path component only: both separator styles, plus NUL just in case.
    let last = raw.rsplit(['/', '\\', '\0']).next().unwrap_or_default();

    let cleaned: String = last
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
        return "file".to_string();
    }

    if trimmed.len() <= MAX_NAME_BYTES {
        return trimmed.to_string();
    }
    // Over-long: keep the extension (it routes files on the device) and
    // truncate the stem on a char boundary.
    let (stem, ext) = split_extension(trimmed);
    let ext = truncate_chars(ext, 20);
    let stem = truncate_chars(stem, MAX_NAME_BYTES - ext.len());
    format!("{stem}{ext}")
}

/// A path in `dir` for `name` that collides neither with existing files nor
/// with `taken` (names already assigned to other files of the same session):
/// `name.gbc`, `name (1).gbc`, `name (2).gbc`, …
pub fn unique_path(dir: &Path, name: &str, taken: &HashSet<PathBuf>) -> PathBuf {
    let free = |p: &PathBuf| !p.exists() && !taken.contains(p);
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
    fn unique_path_suffixes_collisions() {
        let dir = std::env::temp_dir().join(format!("lsretro-files-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut taken = HashSet::new();

        let first = unique_path(&dir, "game.gbc", &taken);
        assert_eq!(first, dir.join("game.gbc"));
        taken.insert(first);

        // Second file of the same session with the same name.
        let second = unique_path(&dir, "game.gbc", &taken);
        assert_eq!(second, dir.join("game (1).gbc"));
        taken.insert(second);

        // A name already on disk collides too.
        std::fs::write(dir.join("save.dat"), b"x").unwrap();
        let third = unique_path(&dir, "save.dat", &taken);
        assert_eq!(third, dir.join("save (1).dat"));

        // No extension.
        taken.insert(dir.join("README"));
        let fourth = unique_path(&dir, "README", &taken);
        assert_eq!(fourth, dir.join("README (1)"));

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
