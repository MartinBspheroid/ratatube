//! Nerd Font icon support with plain-text fallbacks (PRD 10.12).

use ratatube_domain::config::IconMode;

/// Semantic icon slots used across the interface.
#[derive(Debug, Clone, Copy)]
pub struct Icons {
    pub(super) section_bar: &'static str,
    pub(super) panel_rule: &'static str,
    pub prev: &'static str,
    pub next: &'static str,
    pub pause_btn: &'static str,
    pub(super) play_btn: &'static str,
    pub(super) chevron_l: &'static str,
    pub(super) chevron_r: &'static str,
    pub(super) spectrum_ramp: [&'static str; 5],
    pub dropdown: &'static str,
    pub(super) home: &'static str,
    pub(super) playing: &'static str,
    pub paused: &'static str,
    pub stopped: &'static str,
    pub(super) music: &'static str,
    pub(super) volume: &'static str,
    pub muted: &'static str,
    pub(super) search: &'static str,
    pub(super) playlist: &'static str,
    pub(super) queue: &'static str,
    pub(super) history: &'static str,
    pub(super) shuffle: &'static str,
    pub(super) repeat: &'static str,
    pub(super) error: &'static str,
    pub loading: &'static str,
    pub(super) import: &'static str,
    pub(super) dot: &'static str,
    /// Single-cell "in queue" marker for track-table rows (the wide `queue`
    /// tab icon does not fit the marker column in ASCII mode).
    pub(super) marker_queued: &'static str,
}

/// Nerd Font glyphs.
const NERD: Icons = Icons {
    section_bar: "▎",
    panel_rule: "─",
    prev: "\u{f048}",
    next: "\u{f051}",
    pause_btn: "\u{f04c}",
    play_btn: "\u{f04b}",
    chevron_l: "‹",
    chevron_r: "›",
    spectrum_ramp: ["▁", "▂", "▃", "▅", "▇"],
    dropdown: "▾",
    home: "\u{f015}",
    playing: "\u{f04b}",
    paused: "\u{f04c}",
    stopped: "\u{f04d}",
    music: "\u{f001}",
    volume: "\u{f028}",
    muted: "\u{eee8}",
    search: "\u{f002}",
    playlist: "\u{f0cb9}",
    queue: "\u{f0ca}",
    history: "\u{f1da}",
    shuffle: "\u{f074}",
    repeat: "\u{f01e}",
    error: "\u{f071}",
    loading: "\u{f251}",
    import: "\u{f019}",
    dot: "●",
    marker_queued: "\u{f0ca}",
};

/// ASCII fallbacks; the UI must stay fully understandable in this mode.
const ASCII: Icons = Icons {
    section_bar: "|",
    panel_rule: "-",
    prev: "<",
    next: ">",
    pause_btn: "|",
    play_btn: ">",
    chevron_l: "<",
    chevron_r: ">",
    spectrum_ramp: ["_", ".", "-", "=", "#"],
    dropdown: "v",
    home: "[HOME]",
    playing: "[PLAY]",
    paused: "[PAUSE]",
    stopped: "[STOP]",
    music: "[MUSIC]",
    volume: "[VOL]",
    muted: "[MUTE]",
    search: "[SEARCH]",
    playlist: "[LIST]",
    queue: "[QUEUE]",
    history: "[HIST]",
    shuffle: "[SHUFFLE]",
    repeat: "[REPEAT]",
    error: "[ERROR]",
    loading: "[...]",
    import: "[IMPORT]",
    dot: "*",
    marker_queued: "+",
};

/// Select the active icon set from the configured mode.
///
/// `Auto` must be resolved to a concrete mode first (see
/// [`resolve_icon_mode`]); unresolved it falls back to ASCII.
pub fn icons_for(mode: IconMode) -> Icons {
    match mode {
        IconMode::NerdFont => NERD,
        IconMode::Auto | IconMode::Ascii => ASCII,
    }
}

/// Resolve `Auto` to a concrete icon mode by sniffing the terminal.
///
/// Terminals that ship with capable font fallback (Ghostty, Kitty, WezTerm,
/// iTerm2) get Nerd Font glyphs; everything else keeps the conservative
/// ASCII markers. Explicit config values pass through untouched.
pub fn resolve_icon_mode(configured: IconMode) -> IconMode {
    if configured != IconMode::Auto {
        return configured;
    }
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();
    let known_good = matches!(term_program.as_str(), "ghostty" | "WezTerm" | "iTerm.app")
        || term.contains("kitty")
        || term.contains("ghostty")
        || std::env::var("KITTY_WINDOW_ID").is_ok();
    if known_good {
        IconMode::NerdFont
    } else {
        IconMode::Ascii
    }
}

/// Unicode format characters that are invisible or that reorder the text
/// around them, and that [`char::is_control`] does *not* cover (PRD 19).
///
/// Two attack classes, both reachable from any yt-dlp-supplied title, channel
/// name, or description:
///
/// - Bidi embedding/override/isolate controls let a crafted title render in a
///   deceptive order — the "Trojan Source" class of spoofing. In a track list
///   this lets one row impersonate another.
/// - Zero-width and invisible joiners let two distinct strings render
///   identically, and pad text so width calculations disagree with what the
///   terminal actually draws.
///
/// U+200E (LRM) and U+200F (RLM) are deliberately **not** in this set. They
/// are plain directional *marks* that legitimately occur in Arabic and Hebrew
/// text; stripping them corrupts the display of real RTL titles rather than
/// protecting anyone. The spoofing risk is the override/embedding/isolate
/// set, not the marks — do not "tidy" LRM/RLM into this list.
fn is_deceptive_format_char(c: char) -> bool {
    matches!(
        c,
        // Bidi embedding and override: LRE, RLE, PDF, LRO, RLO.
        '\u{202a}'..='\u{202e}'
        // Bidi isolates: LRI, RLI, FSI, PDI.
        | '\u{2066}'..='\u{2069}'
        // Zero-width space, ZWNJ, ZWJ, word joiner, BOM. These carry no ink
        // in a terminal cell grid, so on this surface they only ever serve to
        // hide or pad content.
        | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}'
    )
}

/// Shared filter behind both public sanitizers, so they cannot drift apart.
///
/// Everything else — combining marks, non-Latin scripts, emoji — is
/// legitimate content and passes through untouched. `keep_newlines` is the
/// only intended difference between the two helpers.
fn is_renderable_char(c: char, keep_newlines: bool) -> bool {
    if is_deceptive_format_char(c) {
        return false;
    }
    !c.is_control() || c == ' ' || (keep_newlines && c == '\n')
}

/// Strip terminal control characters and deceptive Unicode format characters
/// from untrusted strings (PRD 19).
pub(super) fn sanitize_terminal_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| is_renderable_char(*c, false))
        .collect()
}

/// Like [`sanitize_terminal_text`] but preserves line breaks, for multi-line
/// content such as video descriptions. `\r\n` collapses to `\n`.
pub(super) fn sanitize_multiline_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| is_renderable_char(*c, true))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_defaults_to_ascii() {
        let icons = icons_for(IconMode::Auto);
        assert_eq!(icons.playing, "[PLAY]");
    }

    #[test]
    fn nerd_font_mode_uses_glyphs() {
        let icons = icons_for(IconMode::NerdFont);
        assert_ne!(icons.playing, "[PLAY]");
    }

    #[test]
    fn sanitizes_control_characters() {
        let dirty = "bad\u{1b}[2Jtitle\u{7}\u{0}";
        assert_eq!(sanitize_terminal_text(dirty), "bad[2Jtitle");
    }

    #[test]
    fn multiline_keeps_newlines_and_collapses_crlf() {
        let dirty = "line one\r\nline\u{7} two";
        assert_eq!(sanitize_multiline_text(dirty), "line one\nline two");
        // The single-line helper still drops every newline.
        assert_eq!(sanitize_terminal_text(dirty), "line oneline two");
    }

    #[test]
    fn strips_bidi_override_and_isolate_controls() {
        // "Trojan Source"-style payload: RLO makes the visible order lie about
        // the actual code points, so one row can impersonate another.
        let spoof = "safe\u{202e}3pm.exe";
        assert_eq!(sanitize_terminal_text(spoof), "safe3pm.exe");

        for control in [
            '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}',
            '\u{2068}', '\u{2069}',
        ] {
            let dirty = format!("a{control}b");
            assert_eq!(sanitize_terminal_text(&dirty), "ab", "control {control:?}");
            assert_eq!(sanitize_multiline_text(&dirty), "ab", "control {control:?}");
        }
    }

    #[test]
    fn strips_zero_width_characters() {
        for invisible in ['\u{200b}', '\u{200c}', '\u{200d}', '\u{2060}', '\u{feff}'] {
            let dirty = format!("Ar{invisible}tist");
            assert_eq!(
                sanitize_terminal_text(&dirty),
                "Artist",
                "invisible {invisible:?}"
            );
            assert_eq!(
                sanitize_multiline_text(&dirty),
                "Artist",
                "invisible {invisible:?}"
            );
        }

        // Two distinct titles that render identically must not stay distinct.
        assert_eq!(
            sanitize_terminal_text("Rick Astley\u{200b}"),
            sanitize_terminal_text("Rick Astley")
        );
    }

    #[test]
    fn keeps_directional_marks_and_rtl_text() {
        // LRM/RLM are benign marks that real Arabic and Hebrew titles carry.
        let rtl = "\u{200f}أم كلثوم - أنت عمري\u{200e} (live)";
        assert_eq!(sanitize_terminal_text(rtl), rtl);
        assert_eq!(sanitize_multiline_text(rtl), rtl);

        // Combining marks, other scripts and emoji are legitimate content.
        let mixed = "Nový\u{301} zpe\u{30c}v — 東京 — 🎵";
        assert_eq!(sanitize_terminal_text(mixed), mixed);
    }

    #[test]
    fn new_icon_slots_are_single_terminal_cells() {
        use unicode_width::UnicodeWidthStr;

        for icons in [NERD, ASCII] {
            for glyph in [
                icons.section_bar,
                icons.panel_rule,
                icons.prev,
                icons.next,
                icons.pause_btn,
                icons.play_btn,
                icons.chevron_l,
                icons.chevron_r,
                icons.dropdown,
                icons.dot,
                icons.marker_queued,
            ] {
                assert_eq!(glyph.width(), 1, "glyph {glyph:?}");
            }
            for glyph in icons.spectrum_ramp {
                assert_eq!(glyph.width(), 1, "spectrum glyph {glyph:?}");
            }
        }
    }
}
