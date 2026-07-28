//! Auto-routing: match the save directory's existing console folders against a
//! table of ROM extensions. Folders are never invented — a device with no `gba`
//! folder gets no `gba` route — and each device's own naming is reused verbatim.
//!
//! Folder names below come from each firmware's own definitions:
//! - KNULLI (and Batocera upstream): the system keys of
//!   `package/batocera/emulationstation/batocera-es-system/es_systems.yml`,
//!   which are the `roms/` folder names.
//! - ROCKNIX: the `Game Path` column of its per-system documentation
//!   (`docs/systems/*.md` in `ROCKNIX/rocknix.org`).
//! - muOS: the keys of `share/info/name/folder.json` in `MustardOS/internal`,
//!   its folder-name-to-display-name map.
//!
//! The three disagree, hence the candidate lists: Mega Drive is `megadrive` on
//! KNULLI, `genesis` on ROCKNIX, `md` on muOS; WonderSwan is `wswan`/`wswanc`,
//! `wonderswan`, and `ws`/`wsc` respectively.

use std::collections::BTreeMap;
use std::path::Path;

/// `(extension, folder names to try in order)`.
///
/// Only extensions that identify one console are listed. Excluded: `zip`, `7z`,
/// `iso`, `bin`, `cue`, `chd` (dozens of systems each), `md` (Markdown too),
/// `d64` (five Commodore machines), `adf` (two Amiga models, Archimedes, BBC),
/// `cso` (PSP, PS2), `rvz` (GameCube, Wii).
///
/// Candidates run from the most specific name to the most general so a device
/// carrying both `gb` and `gbc` routes `.gbc` to `gbc`.
const CONSOLE_FOLDERS: &[(&str, &[&str])] = &[
    ("32x", &["sega32x"]),
    ("3ds", &["3ds"]),
    ("a26", &["atari2600", "a2600", "atari"]),
    ("a78", &["atari7800", "a7800"]),
    ("col", &["colecovision", "coleco"]),
    ("fds", &["fds"]),
    ("gb", &["gb"]),
    ("gba", &["gba"]),
    ("gbc", &["gbc", "gb"]),
    // `.gcm` is GameCube's own image format; `.rvz` is Dolphin's, for both
    // GameCube and Wii, so it stays out.
    ("gcm", &["gamecube", "gc"]),
    ("gdi", &["dreamcast", "dc"]),
    ("gen", &["megadrive", "genesis", "md"]),
    ("gg", &["gamegear", "gg"]),
    ("int", &["intellivision", "intv"]),
    ("j64", &["atarijaguar", "jaguar"]),
    ("jag", &["atarijaguar", "jaguar"]),
    ("lnx", &["atarilynx", "lynx"]),
    ("min", &["pokemini", "poke"]),
    ("n64", &["n64"]),
    ("nds", &["nds", "ds"]),
    ("nes", &["nes", "fc"]),
    // `.ngc` is a Neo Geo Pocket Color cart, not a GameCube image.
    ("ngc", &["ngpc", "ngp"]),
    ("ngp", &["ngp"]),
    ("nsp", &["switch"]),
    ("p8", &["pico8", "pico-8"]),
    ("pce", &["pcengine", "pce", "tg16", "turbografx"]),
    ("psvita", &["psvita", "vita"]),
    ("sfc", &["snes", "sfc"]),
    ("sgx", &["supergrafx"]),
    ("smc", &["snes", "sfc"]),
    ("smd", &["megadrive", "genesis", "md"]),
    ("sms", &["mastersystem", "sms", "ms"]),
    ("v64", &["n64"]),
    ("vb", &["virtualboy", "vb"]),
    ("wbfs", &["wii"]),
    ("ws", &["wswan", "wonderswan", "ws"]),
    ("wsc", &["wswanc", "wonderswan", "wsc"]),
    ("xci", &["switch"]),
    ("z64", &["n64"]),
];

/// `ext -> folder name` for the console folders present in `save_dir`. Empty
/// when the directory is unreadable or holds none of them — the desktop case,
/// where everything keeps landing in `save_dir`.
pub fn detect(save_dir: &Path) -> BTreeMap<String, String> {
    let folders = folders_by_key(save_dir);
    if folders.is_empty() {
        return BTreeMap::new();
    }
    CONSOLE_FOLDERS
        .iter()
        .filter_map(|(ext, candidates)| {
            let folder = candidates.iter().find_map(|c| folders.get(*c))?;
            Some(((*ext).to_string(), folder.clone()))
        })
        .collect()
}

/// Subdirectory names of `dir` indexed by every key a candidate can match: the
/// lowercased name, plus its trailing `(...)` code when it has one — SD-card
/// templates commonly rename folders to `Nintendo Game Boy Advance (GBA)`.
/// Sorted first so a key claimed by two folders resolves the same on every run —
/// `read_dir` order is arbitrary.
fn folders_by_key(dir: &Path) -> BTreeMap<String, String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return BTreeMap::new();
    };
    // `path().is_dir()` over `file_type()`: it follows symlinks, which the
    // handheld firmwares use to point ROM folders at a second card.
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();

    let mut by_key = BTreeMap::new();
    for name in names {
        let keys = std::iter::once(name.to_ascii_lowercase()).chain(parenthesized_code(&name));
        for key in keys {
            by_key.entry(key).or_insert_with(|| name.clone());
        }
    }
    by_key
}

/// The code in a trailing `(...)`, lowercased:
/// `Nintendo Game Boy Advance (GBA)` -> `gba`.
fn parenthesized_code(name: &str) -> Option<String> {
    let inner = name.trim_end().strip_suffix(')')?;
    let open = inner.rfind('(')?;
    let code = inner[open + 1..].trim();
    (!code.is_empty()).then(|| code.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir_with(tag: &str, folders: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "retsend-routes-{tag}-{}",
            crate::net::protocol::random_token(4)
        ));
        for folder in folders {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
        }
        dir
    }

    /// KNULLI/Batocera naming.
    #[test]
    fn routes_knulli_folders() {
        let dir = temp_dir_with("knulli", &["gba", "snes", "megadrive", "wswan", "lynx"]);
        let routes = detect(&dir);

        assert_eq!(routes.get("gba").map(String::as_str), Some("gba"));
        assert_eq!(routes.get("sfc").map(String::as_str), Some("snes"));
        assert_eq!(routes.get("smc").map(String::as_str), Some("snes"));
        assert_eq!(routes.get("gen").map(String::as_str), Some("megadrive"));
        assert_eq!(routes.get("ws").map(String::as_str), Some("wswan"));
        assert_eq!(routes.get("lnx").map(String::as_str), Some("lynx"));
        // No folder, no route: nothing is created on the user's behalf.
        assert!(!routes.contains_key("nes"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// ROCKNIX names Mega Drive `genesis` and keeps one WonderSwan folder.
    #[test]
    fn routes_rocknix_folders() {
        let dir = temp_dir_with("rocknix", &["genesis", "wonderswan", "coleco", "atarilynx"]);
        let routes = detect(&dir);

        assert_eq!(routes.get("gen").map(String::as_str), Some("genesis"));
        assert_eq!(routes.get("ws").map(String::as_str), Some("wonderswan"));
        assert_eq!(routes.get("wsc").map(String::as_str), Some("wonderswan"));
        assert_eq!(routes.get("col").map(String::as_str), Some("coleco"));
        assert_eq!(routes.get("lnx").map(String::as_str), Some("atarilynx"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// muOS uses the short aliases of its `folder.json`.
    #[test]
    fn routes_muos_folders() {
        let dir = temp_dir_with("muos", &["md", "ms", "fc", "ds", "vb", "intv"]);
        let routes = detect(&dir);

        assert_eq!(routes.get("gen").map(String::as_str), Some("md"));
        assert_eq!(routes.get("sms").map(String::as_str), Some("ms"));
        assert_eq!(routes.get("nes").map(String::as_str), Some("fc"));
        assert_eq!(routes.get("nds").map(String::as_str), Some("ds"));
        assert_eq!(routes.get("vb").map(String::as_str), Some("vb"));
        assert_eq!(routes.get("int").map(String::as_str), Some("intv"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn routes_renamed_folders_by_their_code() {
        let dir = temp_dir_with(
            "renamed",
            &[
                "Nintendo Game Boy Advance (GBA)",
                "Sega Mega Drive - Genesis (MD)",
            ],
        );
        let routes = detect(&dir);

        assert_eq!(
            routes.get("gba").map(String::as_str),
            Some("Nintendo Game Boy Advance (GBA)")
        );
        // Matched through the `(MD)` code, and the folder keeps its real name.
        assert_eq!(
            routes.get("gen").map(String::as_str),
            Some("Sega Mega Drive - Genesis (MD)")
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn folder_match_is_case_insensitive() {
        let dir = temp_dir_with("case", &["GBA"]);
        assert_eq!(detect(&dir).get("gba").map(String::as_str), Some("GBA"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_more_specific_folder_wins() {
        let dir = temp_dir_with("specific", &["gb", "gbc"]);
        let routes = detect(&dir);
        assert_eq!(routes.get("gbc").map(String::as_str), Some("gbc"));
        assert_eq!(routes.get("gb").map(String::as_str), Some("gb"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ambiguous_extensions_get_no_route() {
        let dir = temp_dir_with("ambiguous", &["megadrive", "psx", "snes", "amiga", "c64"]);
        let routes = detect(&dir);

        // Folders they could plausibly point at exist; they still stay out.
        for ext in ["md", "zip", "7z", "chd", "cue", "iso", "bin", "adf", "d64"] {
            assert!(!routes.contains_key(ext), "{ext} should not be routed");
        }

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_console_folders_means_no_routes() {
        let dir = temp_dir_with("plain", &["Documents"]);
        assert!(detect(&dir).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(detect(Path::new("/nonexistent/retsend-save-dir")).is_empty());
    }
}
