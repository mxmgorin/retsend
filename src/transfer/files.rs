//! Filename hygiene for received files. Senders control `fileName` byte for
//! byte, so this is a security boundary: strip control characters and
//! FAT-illegal characters (handheld SD cards are FAT), keep every component a
//! name of its own, and never let a path escape the save directory.
//!
//! And the way out: the directory listing the browser and a folder send share,
//! and the walk turning picked paths into the flat list the protocol carries.

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
/// Most files one send may carry. A receiver caps the prepare-upload body
/// (ours at 1 MiB) and every file spends a couple of hundred bytes of it.
const MAX_SEND_FILES: usize = 2048;
/// Deepest folder level a picked directory is walked to.
const MAX_SEND_DEPTH: usize = 16;

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

pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    /// Files only; 0 for directories.
    pub size: u64,
}

/// Directory listing: dirs first, case-insensitive name order, dotfiles
/// hidden, symlinks skipped (a looped symlink tree on an SD card must not hang
/// navigation, nor a folder send).
pub fn list_dir(dir: &Path) -> std::io::Result<Vec<DirEntry>> {
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
        entries.push(DirEntry {
            path: entry.path(),
            is_dir: meta.is_dir(),
            size: if meta.is_dir() { 0 } else { meta.len() },
            name,
        });
    }
    // Cached key: one lowercase per entry, not two per comparison.
    entries.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
    Ok(entries)
}

/// One file of an outbound send. Protocol v2 has no directory of its own, so
/// the files of a sent folder carry their relative path in `name`.
pub struct SendFile {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// What a set of picked sources expands to.
#[derive(Default)]
pub struct SendList {
    pub files: Vec<SendFile>,
    pub bytes: u64,
    /// The walk left something out: [`MAX_SEND_FILES`], [`MAX_SEND_DEPTH`], or
    /// a folder that could not be read.
    pub partial: bool,
}

impl SendList {
    fn push(&mut self, path: PathBuf, name: String, size: u64) {
        self.bytes += size;
        self.files.push(SendFile { path, name, size });
    }
}

/// Is `path` strictly inside `dir`?
pub fn is_inside(dir: &Path, path: &Path) -> bool {
    path != dir && path.starts_with(dir)
}

/// The last component of `path`, or the whole path when it has none.
pub fn base_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The files picked sources carry: a file itself, a directory its tree under
/// the directory's own name. A source inside another picked directory is
/// dropped rather than sent twice; a missing one errors.
pub fn expand_sources(sources: &[PathBuf]) -> std::io::Result<SendList> {
    let mut picked: Vec<&Path> = sources.iter().map(PathBuf::as_path).collect();
    picked.sort_unstable();
    picked.dedup();
    let dirs: Vec<&Path> = picked.iter().copied().filter(|p| p.is_dir()).collect();

    let mut list = SendList::default();
    for source in picked {
        if dirs.iter().any(|dir| is_inside(dir, source)) {
            continue;
        }
        let meta = std::fs::metadata(source)?;
        if meta.is_dir() {
            walk_into(source, &base_name(source), 1, &mut list);
        } else {
            list.push(source.to_path_buf(), base_name(source), meta.len());
        }
    }
    Ok(list)
}

/// A folder's files, walked eagerly so a browser can total them before the send.
pub fn walk_folder(dir: &Path) -> SendList {
    let mut list = SendList::default();
    walk_into(dir, &base_name(dir), 1, &mut list);
    list
}

/// Files under `dir` named `<prefix>/<name>`, over [`list_dir`] — so a folder
/// sends what its listing showed, dotfiles and symlinks in neither.
fn walk_into(dir: &Path, prefix: &str, depth: usize, list: &mut SendList) {
    let entries = match list_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("`{}` left out of the send: {e}", dir.display());
            list.partial = true;
            return;
        }
    };
    for entry in entries {
        if list.files.len() >= MAX_SEND_FILES {
            list.partial = true;
            return;
        }
        let name = format!("{prefix}/{}", entry.name);
        if !entry.is_dir {
            list.push(entry.path, name, entry.size);
        } else if depth < MAX_SEND_DEPTH {
            walk_into(&entry.path, &name, depth + 1, list);
        } else {
            log::warn!("`{}` is nested too deep to send", entry.path.display());
            list.partial = true;
        }
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

    /// `roms/` holding `gb/zelda.gbc`, `gb/saves/zelda.sav` and `notes.txt`,
    /// plus a dotfile and an empty folder the walk must pass over.
    fn send_tree() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "retsend-walk-{}",
            crate::net::protocol::random_token(4)
        ));
        std::fs::create_dir_all(root.join("roms/gb/saves")).unwrap();
        std::fs::create_dir_all(root.join("roms/empty")).unwrap();
        std::fs::write(root.join("roms/gb/zelda.gbc"), vec![0u8; 100]).unwrap();
        std::fs::write(root.join("roms/gb/saves/zelda.sav"), vec![0u8; 8]).unwrap();
        std::fs::write(root.join("roms/notes.txt"), b"hi").unwrap();
        std::fs::write(root.join("roms/.hidden"), b"x").unwrap();
        root
    }

    fn names(list: &SendList) -> Vec<&str> {
        list.files.iter().map(|f| f.name.as_str()).collect()
    }

    #[test]
    fn a_folder_expands_to_its_tree_under_its_own_name() {
        let root = send_tree();
        let list = expand_sources(&[root.join("roms")]).unwrap();

        // Dirs first, as the listing shows them; the dotfile is not sent.
        assert_eq!(
            names(&list),
            [
                "roms/gb/saves/zelda.sav",
                "roms/gb/zelda.gbc",
                "roms/notes.txt"
            ]
        );
        assert_eq!(list.bytes, 110);
        assert!(!list.partial);
        assert_eq!(list.files[1].path, root.join("roms/gb/zelda.gbc"));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_loose_file_keeps_its_bare_name() {
        let root = send_tree();
        let list = expand_sources(&[root.join("roms/notes.txt")]).unwrap();
        assert_eq!(names(&list), ["notes.txt"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_source_inside_a_picked_folder_is_not_sent_twice() {
        let root = send_tree();
        let list = expand_sources(&[
            root.join("roms/gb/zelda.gbc"),
            root.join("roms"),
            root.join("roms/gb"),
            root.join("roms"), // the same pick twice
        ])
        .unwrap();

        assert_eq!(
            names(&list),
            [
                "roms/gb/saves/zelda.sav",
                "roms/gb/zelda.gbc",
                "roms/notes.txt"
            ]
        );
        assert_eq!(list.bytes, 110);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_empty_folder_expands_to_nothing() {
        let root = send_tree();
        let list = expand_sources(&[root.join("roms/empty")]).unwrap();
        assert!(list.files.is_empty() && !list.partial);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_source_is_an_error() {
        assert!(expand_sources(&[PathBuf::from("/nonexistent/card/roms")]).is_err());
    }

    #[test]
    fn the_walk_stops_at_the_send_cap_and_says_so() {
        let root = std::env::temp_dir().join(format!(
            "retsend-cap-{}",
            crate::net::protocol::random_token(4)
        ));
        let folder = root.join("many");
        std::fs::create_dir_all(&folder).unwrap();
        for i in 0..MAX_SEND_FILES + 10 {
            std::fs::write(folder.join(format!("{i:05}.bin")), b"x").unwrap();
        }

        let list = walk_folder(&folder);
        assert_eq!(list.files.len(), MAX_SEND_FILES);
        assert!(list.partial);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_walk_stops_going_deeper_and_says_so() {
        let root = std::env::temp_dir().join(format!(
            "retsend-deep-{}",
            crate::net::protocol::random_token(4)
        ));
        let deep: PathBuf = (0..MAX_SEND_DEPTH + 2).map(|i| format!("d{i}")).collect();
        std::fs::create_dir_all(root.join("tree").join(&deep)).unwrap();
        std::fs::write(root.join("tree").join(&deep).join("buried.bin"), b"x").unwrap();
        std::fs::write(root.join("tree/top.bin"), b"x").unwrap();

        let list = walk_folder(&root.join("tree"));
        assert_eq!(names(&list), ["tree/top.bin"]);
        assert!(list.partial, "the buried file was left out");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn is_inside_is_strict() {
        let dir = Path::new("/roms/gb");
        assert!(is_inside(dir, Path::new("/roms/gb/zelda.gbc")));
        assert!(is_inside(dir, Path::new("/roms/gb/saves/zelda.sav")));
        assert!(!is_inside(dir, dir), "a folder does not contain itself");
        assert!(!is_inside(dir, Path::new("/roms/gba/x.gba")));
        assert!(!is_inside(dir, Path::new("/roms")));
    }

    #[test]
    fn part_path_appends_suffix() {
        assert_eq!(
            part_path(Path::new("/save/game.gbc")),
            PathBuf::from("/save/game.gbc.part")
        );
    }
}
