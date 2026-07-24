//! Color and style theme. The UI must also work without color (PRD 20).
//!
//! Themes are selected by [`crate::config::ThemeName`], either from
//! `config.json` or live from the ctrl+p settings menu. Palette values match
//! each scheme's published specification (see the enum's doc links).

use ratatui::style::{Color, Modifier, Style};

use crate::config::ThemeName;

/// Central palette resolved from the selected theme.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub truecolor: bool,
    pub bg: Style,
    pub panel_bg: Style,
    pub base: Style,
    pub accent: Style,
    pub accent_alt: Style,
    pub dim: Style,
    pub error: Style,
    pub warning: Style,
    pub orange: Style,
    pub playing: Style,
    pub selected: Style,
    pub header: Style,
    pub border: Style,
    pub border_active: Style,
    pub gauge_filled: Style,
    pub tab_active: Style,
    pub key_chip: Style,
    pub panel_title: Style,
    pub panel_rule: Style,
    pub chip: Style,
    pub success: Style,
    pub link: Style,
    pub value: Style,
    pub label: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::resolve(ThemeName::default())
    }
}

/// Truecolor swatch of one published color scheme, mapped onto the style
/// roles the UI consumes. All values are the scheme's official hex colors.
struct Palette {
    bg: Color,
    panel_bg: Color,
    /// Emphasized foreground: headers, values, selected rows.
    text: Color,
    /// Secondary foreground: chips and de-emphasized copy.
    subtext: Color,
    /// Lowest-emphasis foreground: labels, rules, hints.
    dim: Color,
    accent: Color,
    accent_alt: Color,
    red: Color,
    yellow: Color,
    orange: Color,
    green: Color,
    selected_bg: Color,
    border: Color,
}

/// Catppuccin Mocha: mantle canvas, base panels, mauve/pink accents.
const CATPPUCCIN_MOCHA: Palette = Palette {
    bg: Color::Rgb(24, 24, 37),
    panel_bg: Color::Rgb(30, 30, 46),
    text: Color::Rgb(205, 214, 244),
    subtext: Color::Rgb(166, 173, 200),
    dim: Color::Rgb(108, 112, 134),
    accent: Color::Rgb(203, 166, 247),
    accent_alt: Color::Rgb(245, 194, 231),
    red: Color::Rgb(243, 139, 168),
    yellow: Color::Rgb(249, 226, 175),
    orange: Color::Rgb(250, 179, 135),
    green: Color::Rgb(166, 227, 161),
    selected_bg: Color::Rgb(69, 71, 90),
    border: Color::Rgb(88, 91, 112),
};

/// Solarized Dark: base03 canvas, base02 panels, blue/cyan accents.
const SOLARIZED_DARK: Palette = Palette {
    bg: Color::Rgb(0, 43, 54),
    panel_bg: Color::Rgb(7, 54, 66),
    text: Color::Rgb(147, 161, 161),
    subtext: Color::Rgb(131, 148, 150),
    dim: Color::Rgb(88, 110, 117),
    accent: Color::Rgb(38, 139, 210),
    accent_alt: Color::Rgb(42, 161, 152),
    red: Color::Rgb(220, 50, 47),
    yellow: Color::Rgb(181, 137, 0),
    orange: Color::Rgb(203, 75, 22),
    green: Color::Rgb(133, 153, 0),
    selected_bg: Color::Rgb(88, 110, 117),
    border: Color::Rgb(88, 110, 117),
};

/// Tokyo Night: night canvas, storm panels, blue/purple accents.
const TOKYO_NIGHT: Palette = Palette {
    bg: Color::Rgb(26, 27, 38),
    panel_bg: Color::Rgb(36, 40, 59),
    text: Color::Rgb(192, 202, 245),
    subtext: Color::Rgb(169, 177, 214),
    dim: Color::Rgb(86, 95, 137),
    accent: Color::Rgb(122, 162, 247),
    accent_alt: Color::Rgb(187, 154, 247),
    red: Color::Rgb(247, 118, 142),
    yellow: Color::Rgb(224, 175, 104),
    orange: Color::Rgb(255, 158, 100),
    green: Color::Rgb(158, 206, 106),
    selected_bg: Color::Rgb(40, 52, 87),
    border: Color::Rgb(59, 66, 97),
};

/// Gruvbox dark mode: warm grays with the signature bright orange.
const GRUVBOX_DARK: Palette = Palette {
    bg: Color::Rgb(40, 40, 40),
    panel_bg: Color::Rgb(60, 56, 54),
    text: Color::Rgb(235, 219, 178),
    subtext: Color::Rgb(168, 153, 132),
    dim: Color::Rgb(146, 131, 116),
    accent: Color::Rgb(254, 128, 25),
    accent_alt: Color::Rgb(142, 192, 124),
    red: Color::Rgb(251, 73, 52),
    yellow: Color::Rgb(250, 189, 47),
    orange: Color::Rgb(254, 128, 25),
    green: Color::Rgb(184, 187, 38),
    selected_bg: Color::Rgb(80, 73, 69),
    border: Color::Rgb(102, 92, 84),
};

/// Nord: polar-night surfaces, frost accents, aurora status colors.
const NORD: Palette = Palette {
    bg: Color::Rgb(46, 52, 64),
    panel_bg: Color::Rgb(59, 66, 82),
    text: Color::Rgb(236, 239, 244),
    subtext: Color::Rgb(216, 222, 233),
    dim: Color::Rgb(76, 86, 106),
    accent: Color::Rgb(136, 192, 208),
    accent_alt: Color::Rgb(180, 142, 173),
    red: Color::Rgb(191, 97, 106),
    yellow: Color::Rgb(235, 203, 139),
    orange: Color::Rgb(208, 135, 112),
    green: Color::Rgb(163, 190, 140),
    selected_bg: Color::Rgb(67, 76, 94),
    border: Color::Rgb(76, 86, 106),
};

/// Dracula: the official spec palette with purple/pink accents.
const DRACULA: Palette = Palette {
    bg: Color::Rgb(33, 34, 44),
    panel_bg: Color::Rgb(40, 42, 54),
    text: Color::Rgb(248, 248, 242),
    subtext: Color::Rgb(98, 114, 164),
    dim: Color::Rgb(98, 114, 164),
    accent: Color::Rgb(189, 147, 249),
    accent_alt: Color::Rgb(255, 121, 198),
    red: Color::Rgb(255, 85, 85),
    yellow: Color::Rgb(241, 250, 140),
    orange: Color::Rgb(255, 184, 108),
    green: Color::Rgb(80, 250, 123),
    selected_bg: Color::Rgb(68, 71, 90),
    border: Color::Rgb(98, 114, 164),
};

impl Theme {
    /// Build `name`'s palette after sniffing terminal color capability from
    /// the environment (`COLORTERM`, `NO_COLOR`).
    pub fn resolve(name: ThemeName) -> Self {
        let truecolor = std::env::var("COLORTERM").is_ok_and(|value| {
            matches!(value.to_ascii_lowercase().as_str(), "truecolor" | "24bit")
        }) && std::env::var_os("NO_COLOR").is_none();
        Self::from_preset(name, truecolor)
    }

    /// Build `name`'s palette for a known terminal color capability. Without
    /// truecolor every theme collapses to the shared ANSI fallback (PRD 20).
    pub fn from_preset(name: ThemeName, truecolor: bool) -> Self {
        if !truecolor {
            return Self::from_truecolor(false);
        }
        match name {
            ThemeName::Neon => Self::from_truecolor(true),
            ThemeName::CatppuccinMocha => Self::from_palette(&CATPPUCCIN_MOCHA),
            ThemeName::SolarizedDark => Self::from_palette(&SOLARIZED_DARK),
            ThemeName::TokyoNight => Self::from_palette(&TOKYO_NIGHT),
            ThemeName::GruvboxDark => Self::from_palette(&GRUVBOX_DARK),
            ThemeName::Nord => Self::from_palette(&NORD),
            ThemeName::Dracula => Self::from_palette(&DRACULA),
        }
    }

    /// Map one truecolor palette onto the style roles the UI consumes.
    fn from_palette(palette: &Palette) -> Self {
        Self {
            truecolor: true,
            bg: Style::default().bg(palette.bg),
            panel_bg: Style::default().bg(palette.panel_bg),
            base: Style::default(),
            accent: Style::default().fg(palette.accent),
            accent_alt: Style::default().fg(palette.accent_alt),
            dim: Style::default().fg(palette.dim),
            error: Style::default().fg(palette.red),
            warning: Style::default().fg(palette.yellow),
            orange: Style::default().fg(palette.orange),
            playing: Style::default().fg(palette.green),
            selected: Style::default()
                .bg(palette.selected_bg)
                .fg(palette.text)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(palette.text)
                .add_modifier(Modifier::BOLD),
            border: Style::default().fg(palette.border),
            border_active: Style::default().fg(palette.accent),
            gauge_filled: Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            tab_active: Style::default()
                .fg(palette.bg)
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD),
            key_chip: Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            panel_title: Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            panel_rule: Style::default().fg(palette.dim),
            chip: Style::default().fg(palette.subtext),
            success: Style::default().fg(palette.green),
            link: Style::default().fg(palette.accent),
            value: Style::default().fg(palette.text),
            label: Style::default().fg(palette.dim),
        }
    }

    /// Build the original neon palette for a known terminal color capability.
    pub fn from_truecolor(truecolor: bool) -> Self {
        let accent_color = if truecolor {
            Color::Rgb(34, 211, 238)
        } else {
            Color::Cyan
        };
        let accent_alt_color = if truecolor {
            Color::Rgb(217, 70, 239)
        } else {
            Color::Magenta
        };
        Self {
            truecolor,
            bg: if truecolor {
                Style::default().bg(Color::Rgb(5, 10, 18))
            } else {
                Style::default()
            },
            panel_bg: if truecolor {
                Style::default().bg(Color::Rgb(10, 17, 28))
            } else {
                Style::default()
            },
            base: Style::default(),
            accent: Style::default().fg(accent_color),
            accent_alt: Style::default().fg(accent_alt_color),
            dim: Style::default().fg(Color::DarkGray),
            error: Style::default().fg(Color::Red),
            warning: Style::default().fg(Color::Yellow),
            orange: Style::default().fg(if truecolor {
                Color::Rgb(249, 115, 22)
            } else {
                Color::LightRed
            }),
            playing: Style::default().fg(Color::Green),
            selected: Style::default()
                .bg(if truecolor {
                    Color::Rgb(16, 52, 64)
                } else {
                    Color::DarkGray
                })
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            border: Style::default().fg(Color::Rgb(70, 70, 80)),
            border_active: Style::default().fg(accent_color),
            gauge_filled: Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
            tab_active: Style::default()
                .fg(Color::Black)
                .bg(accent_color)
                .add_modifier(Modifier::BOLD),
            key_chip: Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
            panel_title: Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
            panel_rule: Style::default().fg(Color::DarkGray),
            chip: Style::default().fg(Color::Gray),
            success: Style::default().fg(Color::Green),
            link: Style::default().fg(accent_color),
            value: Style::default().fg(Color::White),
            label: Style::default().fg(Color::DarkGray),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_theme_background_tokens_are_true_no_ops() {
        let theme = Theme::from_truecolor(false);
        assert_eq!(theme.bg, Style::default());
        assert_eq!(theme.panel_bg, Style::default());
        assert!(!theme.truecolor);
    }

    #[test]
    fn truecolor_theme_uses_the_mock_navy_backgrounds() {
        let theme = Theme::from_truecolor(true);
        assert_eq!(theme.bg.bg, Some(Color::Rgb(5, 10, 18)));
        assert_eq!(theme.panel_bg.bg, Some(Color::Rgb(10, 17, 28)));
        assert_eq!(theme.accent.fg, Some(Color::Rgb(34, 211, 238)));
        assert!(theme.truecolor);
    }

    #[test]
    fn neon_preset_is_the_original_truecolor_palette() {
        let preset = Theme::from_preset(ThemeName::Neon, true);
        let original = Theme::from_truecolor(true);
        assert_eq!(preset.bg, original.bg);
        assert_eq!(preset.accent, original.accent);
        assert_eq!(preset.selected, original.selected);
    }

    #[test]
    fn every_preset_collapses_to_the_ansi_fallback_without_truecolor() {
        for name in ThemeName::ALL {
            let theme = Theme::from_preset(name, false);
            assert_eq!(theme.bg, Style::default(), "{name:?}");
            assert_eq!(theme.accent.fg, Some(Color::Cyan), "{name:?}");
            assert!(!theme.truecolor, "{name:?}");
        }
    }

    #[test]
    fn presets_use_their_published_background_and_accent() {
        let cases = [
            (
                ThemeName::CatppuccinMocha,
                Color::Rgb(24, 24, 37),
                Color::Rgb(203, 166, 247),
            ),
            (
                ThemeName::SolarizedDark,
                Color::Rgb(0, 43, 54),
                Color::Rgb(38, 139, 210),
            ),
            (
                ThemeName::TokyoNight,
                Color::Rgb(26, 27, 38),
                Color::Rgb(122, 162, 247),
            ),
            (
                ThemeName::GruvboxDark,
                Color::Rgb(40, 40, 40),
                Color::Rgb(254, 128, 25),
            ),
            (
                ThemeName::Nord,
                Color::Rgb(46, 52, 64),
                Color::Rgb(136, 192, 208),
            ),
            (
                ThemeName::Dracula,
                Color::Rgb(33, 34, 44),
                Color::Rgb(189, 147, 249),
            ),
        ];
        for (name, bg, accent) in cases {
            let theme = Theme::from_preset(name, true);
            assert_eq!(theme.bg.bg, Some(bg), "{name:?}");
            assert_eq!(theme.accent.fg, Some(accent), "{name:?}");
            assert_eq!(theme.tab_active.bg, Some(accent), "{name:?}");
            assert!(theme.truecolor, "{name:?}");
        }
    }
}
