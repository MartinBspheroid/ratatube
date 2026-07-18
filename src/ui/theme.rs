//! Color and style theme. The UI must also work without color (PRD 20).

use ratatui::style::{Color, Modifier, Style};

/// Central palette; theme configuration lands in v1.1.
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
        let truecolor = std::env::var("COLORTERM").is_ok_and(|value| {
            matches!(value.to_ascii_lowercase().as_str(), "truecolor" | "24bit")
        }) && std::env::var_os("NO_COLOR").is_none();
        Self::from_truecolor(truecolor)
    }
}

impl Theme {
    /// Build the palette for a known terminal color capability.
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
}
