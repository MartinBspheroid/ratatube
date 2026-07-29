//! Color and style theme. The UI must also work without color (PRD 20).
//!
//! Themes are selected by [`ratatube_domain::config::ThemeName`], either from
//! `config.json` or live from the ctrl+p settings menu. Palette values match
//! each scheme's published specification (see the enum's doc links).

mod palettes;

use palettes::Palette;
use ratatui::style::{Color, Modifier, Style};

use ratatube_domain::config::ThemeName;

/// Central palette resolved from the selected theme.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub(super) truecolor: bool,
    pub(super) bg: Style,
    pub(super) panel_bg: Style,
    pub(super) base: Style,
    pub(super) accent: Style,
    pub(super) accent_alt: Style,
    pub(super) dim: Style,
    pub(super) error: Style,
    pub(super) warning: Style,
    pub(super) orange: Style,
    pub(super) playing: Style,
    pub(super) selected: Style,
    pub(super) header: Style,
    pub(super) border: Style,
    pub(super) border_active: Style,
    pub(super) gauge_filled: Style,
    pub(super) tab_active: Style,
    pub(super) key_chip: Style,
    pub(super) panel_title: Style,
    pub(super) panel_rule: Style,
    pub(super) chip: Style,
    pub(super) success: Style,
    pub(super) link: Style,
    pub(super) value: Style,
    pub(super) label: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::resolve(ThemeName::default())
    }
}

impl Theme {
    /// Build `name`'s palette after sniffing terminal color capability from
    /// the environment (`COLORTERM`, `NO_COLOR`).
    pub(super) fn resolve(name: ThemeName) -> Self {
        let truecolor = std::env::var("COLORTERM").is_ok_and(|value| {
            matches!(value.to_ascii_lowercase().as_str(), "truecolor" | "24bit")
        }) && std::env::var_os("NO_COLOR").is_none();
        Self::from_preset(name, truecolor)
    }

    /// Build `name`'s palette for a known terminal color capability. Without
    /// truecolor every theme collapses to the shared ANSI fallback (PRD 20).
    pub(super) fn from_preset(name: ThemeName, truecolor: bool) -> Self {
        if !truecolor {
            return Self::from_truecolor(false);
        }
        match palettes::palette_for(name) {
            Some(palette) => Self::from_palette(palette),
            None => Self::from_truecolor(true),
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
    pub(super) fn from_truecolor(truecolor: bool) -> Self {
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
            (
                ThemeName::CatppuccinLatte,
                Color::Rgb(230, 233, 239),
                Color::Rgb(136, 57, 239),
            ),
            (
                ThemeName::SolarizedLight,
                Color::Rgb(253, 246, 227),
                Color::Rgb(38, 139, 210),
            ),
            (
                ThemeName::Alucard,
                Color::Rgb(255, 251, 235),
                Color::Rgb(100, 74, 201),
            ),
            (
                ThemeName::FlexokiLight,
                Color::Rgb(255, 252, 240),
                Color::Rgb(32, 94, 166),
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

    #[test]
    fn every_variant_resolves_to_a_distinct_truecolor_palette() {
        use ratatube_domain::config::ThemeMode;
        let mut backgrounds = std::collections::HashSet::new();
        for name in ThemeName::ALL {
            let theme = Theme::from_preset(name, true);
            let bg = theme.bg.bg.expect("truecolor themes set a background");
            assert!(
                backgrounds.insert(format!("{bg:?}")),
                "duplicate bg {name:?}"
            );
            // Light variants sit on light backgrounds, dark on dark.
            if let Color::Rgb(r, g, b) = bg {
                let luma = u16::from(r) + u16::from(g) + u16::from(b);
                match name.mode() {
                    ThemeMode::Dark => assert!(luma < 384, "{name:?} bg too light"),
                    ThemeMode::Light => assert!(luma > 384, "{name:?} bg too dark"),
                }
            }
        }
    }
}
