//! The ctrl+p settings menu overlay: an Appearance tab listing every theme
//! with live-preview swatches, and a General tab for icon and resume modes.

use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::app::state::{SettingsState, SettingsTab};
use crate::config::{IconMode, ResumeMode, ThemeName};
use crate::ui::{centered_rect, theme};

const MODAL_WIDTH: u16 = 52;
/// Widest row label ("Catppuccin Mocha"), plus marker spacing.
const LABEL_WIDTH: usize = 18;

/// Render the settings menu when it is open.
pub(super) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    settings: &SettingsState,
    theme: &theme::Theme,
) {
    let mut lines = vec![tabs_line(settings.tab, theme), Line::from("")];
    match settings.tab {
        SettingsTab::Appearance => {
            for (index, name) in ThemeName::ALL.into_iter().enumerate() {
                lines.push(theme_row(name, index == settings.selected, theme));
            }
        }
        SettingsTab::General => {
            lines.push(value_row(
                "Icons",
                IconMode::label(settings.icons),
                settings.selected == 0,
                theme,
            ));
            lines.push(value_row(
                "Resume on launch",
                ResumeMode::label(settings.resume),
                settings.selected == 1,
                theme,
            ));
        }
    }
    lines.push(Line::from(""));
    lines.push(hints_line(settings.tab, theme));

    let height = (lines.len() as u16).saturating_add(2);
    let modal_area = centered_rect(area, MODAL_WIDTH, height);
    frame.render_widget(Clear, modal_area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(theme.border_active)
                .title(" Settings "),
        ),
        modal_area,
    );
}

/// Tab strip; the active tab uses the accent chip style.
fn tabs_line(active: SettingsTab, theme: &theme::Theme) -> Line<'static> {
    let tab = |label: &'static str, is_active: bool| {
        Span::styled(
            format!(" {label} "),
            if is_active {
                theme.tab_active
            } else {
                theme.chip
            },
        )
    };
    Line::from(vec![
        Span::raw(" "),
        tab("Appearance", active == SettingsTab::Appearance),
        Span::raw(" "),
        tab("General", active == SettingsTab::General),
    ])
}

/// One selectable theme with swatches drawn in that theme's own colors.
fn theme_row(name: ThemeName, is_selected: bool, theme: &theme::Theme) -> Line<'static> {
    let label_style = if is_selected {
        theme.selected
    } else {
        theme.value
    };
    let marker = if is_selected { " ❯ " } else { "   " };
    let preview = theme::Theme::from_preset(name, theme.truecolor);
    let mut spans = vec![
        Span::styled(marker.to_string(), theme.accent),
        Span::styled(format!("{:LABEL_WIDTH$}", name.label()), label_style),
        Span::raw("  "),
    ];
    for style in [
        preview.accent,
        preview.accent_alt,
        preview.playing,
        preview.orange,
        preview.warning,
    ] {
        spans.push(Span::styled("██", style));
    }
    Line::from(spans)
}

/// One label/value row on the General tab.
fn value_row(
    label: &'static str,
    value: &'static str,
    is_selected: bool,
    theme: &theme::Theme,
) -> Line<'static> {
    let label_style = if is_selected {
        theme.selected
    } else {
        theme.value
    };
    let marker = if is_selected { " ❯ " } else { "   " };
    Line::from(vec![
        Span::styled(marker.to_string(), theme.accent),
        Span::styled(format!("{label:LABEL_WIDTH$}"), label_style),
        Span::raw("  "),
        Span::styled("‹ ", theme.dim),
        Span::styled(value.to_string(), theme.accent),
        Span::styled(" ›", theme.dim),
    ])
}

/// Key hints matching the active tab's available actions.
fn hints_line(tab: SettingsTab, theme: &theme::Theme) -> Line<'static> {
    let mut spans = vec![
        Span::styled(" Tab ", theme.key_chip),
        Span::styled("tabs  ", theme.dim),
        Span::styled(" j/k ", theme.key_chip),
        Span::styled("select  ", theme.dim),
    ];
    if tab == SettingsTab::General {
        spans.push(Span::styled(" h/l ", theme.key_chip));
        spans.push(Span::styled("change  ", theme.dim));
    }
    spans.push(Span::styled(" Enter ", theme.key_chip));
    spans.push(Span::styled("save  ", theme.dim));
    spans.push(Span::styled(" Esc ", theme.key_chip));
    spans.push(Span::styled("cancel", theme.dim));
    Line::from(spans)
}
