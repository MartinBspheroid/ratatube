//! The ctrl+p settings menu overlay: an Appearance tab listing every theme
//! family with live-preview swatches and a dark/light switch, and a General
//! tab for icon and resume modes.

use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::app::state::{SettingsState, SettingsTab};
use crate::config::{IconMode, ResumeMode, ThemeFamily, ThemeMode, ThemeName};
use crate::ui::{centered_rect, theme};

const MODAL_WIDTH: u16 = 58;
/// Widest family label ("Tokyo Night"), plus marker spacing.
const LABEL_WIDTH: usize = 13;
/// Widest variant label ("Dracula").
const VARIANT_WIDTH: usize = 8;
/// Lines outside the row list: tabs, blank, blank, hints.
const CHROME_ROWS: u16 = 4;

/// Render the settings menu when it is open.
pub(super) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    settings: &SettingsState,
    theme: &theme::Theme,
    active_theme: ThemeName,
) {
    let mode = active_theme.mode();
    let mut lines = vec![tabs_line(settings.tab, mode, theme), Line::from("")];
    match settings.tab {
        SettingsTab::Appearance => {
            let families = ThemeFamily::ALL;
            // Window the list around the selection on short terminals.
            let capacity = area
                .height
                .saturating_sub(2 + CHROME_ROWS)
                .clamp(3, families.len() as u16) as usize;
            let start = settings
                .selected
                .saturating_sub(capacity - 1)
                .min(families.len() - capacity);
            for (index, family) in families.iter().enumerate().skip(start).take(capacity) {
                lines.push(family_row(*family, mode, index == settings.selected, theme));
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

/// Tab strip; the Appearance tab also shows the active dark/light mode.
fn tabs_line(active: SettingsTab, mode: ThemeMode, theme: &theme::Theme) -> Line<'static> {
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
    let mut spans = vec![
        Span::raw(" "),
        tab("Appearance", active == SettingsTab::Appearance),
        Span::raw(" "),
        tab("General", active == SettingsTab::General),
    ];
    if active == SettingsTab::Appearance {
        spans.push(Span::raw("   "));
        spans.push(Span::styled("‹ ", theme.dim));
        spans.push(Span::styled(mode.label().to_string(), theme.accent));
        spans.push(Span::styled(" ›", theme.dim));
    }
    Line::from(spans)
}

/// One selectable family in the active mode, with swatches drawn in that
/// variant's own colors.
fn family_row(
    family: ThemeFamily,
    mode: ThemeMode,
    is_selected: bool,
    theme: &theme::Theme,
) -> Line<'static> {
    let variant = family.variant(mode);
    let label_style = if is_selected {
        theme.selected
    } else {
        theme.value
    };
    let marker = if is_selected { " ❯ " } else { "   " };
    let preview = theme::Theme::from_preset(variant, theme.truecolor);
    let mut spans = vec![
        Span::styled(marker.to_string(), theme.accent),
        Span::styled(format!("{:LABEL_WIDTH$}", family.label()), label_style),
        Span::styled(
            format!("{:VARIANT_WIDTH$}", variant.variant_label()),
            theme.chip,
        ),
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
    let change = match tab {
        SettingsTab::Appearance => "mode ",
        SettingsTab::General => "change ",
    };
    Line::from(vec![
        Span::styled(" Tab ", theme.key_chip),
        Span::styled("tabs ", theme.dim),
        Span::styled(" j/k ", theme.key_chip),
        Span::styled("select ", theme.dim),
        Span::styled(" h/l ", theme.key_chip),
        Span::styled(change, theme.dim),
        Span::styled(" Enter ", theme.key_chip),
        Span::styled("save ", theme.dim),
        Span::styled(" Esc ", theme.key_chip),
        Span::styled("cancel", theme.dim),
    ])
}
