//! Nerd Font icon support with plain-text fallbacks (PRD 10.12).

use crate::config::IconMode;

/// Semantic icon slots used across the interface.
#[derive(Debug, Clone, Copy)]
pub struct Icons {
    pub playing: &'static str,
    pub paused: &'static str,
    pub stopped: &'static str,
    pub music: &'static str,
    pub volume: &'static str,
    pub muted: &'static str,
    pub search: &'static str,
    pub playlist: &'static str,
    pub queue: &'static str,
    pub history: &'static str,
    pub shuffle: &'static str,
    pub repeat: &'static str,
    pub error: &'static str,
    pub loading: &'static str,
    pub import: &'static str,
}

/// Nerd Font glyphs.
const NERD: Icons = Icons {
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
};

/// ASCII fallbacks; the UI must stay fully understandable in this mode.
const ASCII: Icons = Icons {
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

/// Strip terminal control characters from untrusted strings (PRD 19).
pub fn sanitize_terminal_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .collect()
}

/// Like [`sanitize_terminal_text`] but preserves line breaks, for multi-line
/// content such as video descriptions. `\r\n` collapses to `\n`.
pub fn sanitize_multiline_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == ' ' || *c == '\n')
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
}
