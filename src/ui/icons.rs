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
/// `auto` cannot reliably detect Nerd Fonts, so it uses the conservative
/// ASCII default; users override with `nerd-font` (PRD 10.12).
pub fn icons_for(mode: IconMode) -> Icons {
    match mode {
        IconMode::NerdFont => NERD,
        IconMode::Auto | IconMode::Ascii => ASCII,
    }
}

/// Strip terminal control characters from untrusted strings (PRD 19).
pub fn sanitize_terminal_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
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
