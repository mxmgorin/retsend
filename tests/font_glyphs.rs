//! Guard against tofu: every non-ASCII glyph the UI paints must resolve in the
//! exact font stack `ui::theme::apply` builds. egui's bundled emoji fonts cover
//! far fewer codepoints than their cmaps suggest (✓ ✗ ✔ ✖ 🕘 all render as
//! empty boxes), so glyph choices are verified here rather than by eye.

use egui_sdl2::egui;
use egui_sdl2::egui::epaint::text::{FontDefinitions, Fonts, TextOptions};

/// Every glyph the UI relies on, with where it's used (for failure messages).
const USED: &[(char, &str)] = &[
    ('√', "history/transfer: completed"),
    ('×', "history/transfer: failed/cancelled"),
    ('↑', "tab: Send"),
    ('↓', "tab: Receive"),
    ('↺', "tab: History"),
    ('⚙', "tab: Settings"),
    ('←', "settings: back hint"),
    ('→', "prompt/home: save path"),
    ('▏', "osk: text cursor"),
    ('°', "wordmark"),
    ('·', "detail separators"),
    ('—', "misc dashes"),
    ('…', "truncation / waiting"),
];

#[test]
fn all_ui_glyphs_render() {
    // Mirror ui::theme::apply: default fonts + Hack appended to Proportional.
    let mut defs = FontDefinitions::default();
    defs.families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .push("Hack".to_owned());

    let mut fonts = Fonts::new(TextOptions::default(), defs);
    fonts.begin_pass(TextOptions::default());
    let id = egui::FontId::proportional(16.0);

    let tofu: Vec<_> = USED
        .iter()
        .filter(|(c, _)| !fonts.has_glyph(&id, *c))
        .map(|(c, where_)| format!("U+{:04X} {c:?} ({where_})", *c as u32))
        .collect();

    assert!(
        tofu.is_empty(),
        "glyphs render as tofu:\n{}",
        tofu.join("\n")
    );
}
