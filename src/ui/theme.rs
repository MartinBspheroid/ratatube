//! Color and style theme. The UI must also work without color (PRD 20).

use ratatui::style::{Color, Modifier, Style};

/// Central palette; theme configuration lands in v1.1.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub base: Style,
    pub accent: Style,
    pub accent_alt: Style,
    pub dim: Style,
    pub error: Style,
    pub warning: Style,
    pub playing: Style,
    pub selected: Style,
    pub header: Style,
    pub border: Style,
    pub border_active: Style,
    pub gauge_filled: Style,
    pub tab_active: Style,
    pub key_chip: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            base: Style::default(),
            accent: Style::default().fg(Color::Cyan),
            accent_alt: Style::default().fg(Color::Magenta),
            dim: Style::default().fg(Color::DarkGray),
            error: Style::default().fg(Color::Red),
            warning: Style::default().fg(Color::Yellow),
            playing: Style::default().fg(Color::Green),
            selected: Style::default()
                .bg(Color::Rgb(30, 60, 70))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            header: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            border: Style::default().fg(Color::Rgb(70, 70, 80)),
            border_active: Style::default().fg(Color::Cyan),
            gauge_filled: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            tab_active: Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            key_chip: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        }
    }
}
