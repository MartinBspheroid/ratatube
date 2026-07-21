//! Shared cyberpunk UI primitives used by responsive views.

use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use unicode_width::UnicodeWidthStr;

use crate::ui::icons::Icons;
use crate::ui::theme::Theme;

mod playback;
mod track_table;

pub use playback::playback_summary;
pub use track_table::{
    TrackFlags, TrackRow, TrackTableLayout, header_row, marker_legend, message_row, track_flags,
    track_row,
};

/// Draw a dedicated title line and return the content rectangle below it.
pub fn section_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
    theme: &Theme,
    icons: &Icons,
) -> Rect {
    if area.is_empty() {
        return area;
    }
    frame.buffer_mut().set_style(area, theme.panel_bg);
    // One focus grammar: the marker glyph never changes (the play glyph is
    // reserved for the playing track); focus is signaled by style alone.
    let marker = icons.section_bar;
    let title = title.to_ascii_uppercase();
    let occupied = marker.width() + 1 + title.width() + 1;
    let rule = icons
        .panel_rule
        .repeat((area.width as usize).saturating_sub(occupied));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                marker,
                if focused {
                    theme.panel_title
                } else {
                    theme.dim
                },
            ),
            Span::raw(" "),
            Span::styled(
                title,
                if focused {
                    theme.panel_title.add_modifier(Modifier::REVERSED)
                } else {
                    theme.dim
                },
            ),
            Span::raw(" "),
            Span::styled(rule, theme.panel_rule),
        ])),
        Rect { height: 1, ..area },
    );
    Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    }
}

/// Build a link label and optional, truthful accelerator chip.
pub fn header_link<'a>(label: &'a str, key: Option<char>, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = vec![Span::styled(
        label,
        if key.is_some() { theme.link } else { theme.dim },
    )];
    if let Some(key) = key {
        spans.push(Span::styled(format!(" · {key}"), theme.key_chip));
    }
    spans
}

/// Render compact bracketed metadata chips.
pub fn chips(items: &[String], theme: &Theme) -> Line<'static> {
    Line::from(
        items
            .iter()
            .flat_map(|item| {
                [
                    Span::styled(format!("[{item}]"), theme.chip),
                    Span::raw(" "),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

/// Build aligned label/value rows.
pub fn key_value_rows(
    pairs: &[(String, String)],
    right_align: bool,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let label_width = pairs
        .iter()
        .map(|(label, _)| label.width())
        .max()
        .unwrap_or(0);
    pairs
        .iter()
        .map(|(label, value)| {
            let value = if right_align {
                format!("{value:>16}")
            } else {
                value.clone()
            };
            Line::from(vec![
                Span::styled(format!("{label:<label_width$}"), theme.label),
                Span::raw("  "),
                Span::styled(value, theme.value),
            ])
        })
        .collect()
}

/// Content and semantic state for a numbered list row.
pub struct NumberedRow<'a> {
    pub index: usize,
    pub title: &'a str,
    pub right_columns: &'a [String],
    pub playing: bool,
    pub selected: bool,
}

/// Build a width-safe numbered row with right-aligned secondary columns.
pub fn numbered_row(
    row: NumberedRow<'_>,
    width: usize,
    theme: &Theme,
    icons: &Icons,
) -> Line<'static> {
    let prefix = format!(
        "{} {:02} ",
        if row.playing { icons.play_btn } else { " " },
        row.index + 1
    );
    let right = row.right_columns.join("  ");
    let budget = width
        .saturating_sub(prefix.width())
        .saturating_sub(right.width())
        .saturating_sub(2);
    let title = crate::ui::widgets::truncate_end(row.title, budget);
    let padding = width
        .saturating_sub(prefix.width() + title.width() + right.width())
        .max(1);
    Line::from(vec![
        Span::styled(
            prefix,
            if row.playing {
                theme.success
            } else {
                theme.dim
            },
        ),
        Span::styled(
            title,
            if row.selected {
                theme.selected
            } else {
                theme.base
            },
        ),
        Span::raw(" ".repeat(padding)),
        Span::styled(right, theme.dim),
    ])
}

/// Build a compact one-row button with its actual accelerator.
pub fn button<'a>(label: &'a str, key: char, style: Style, theme: &Theme) -> Vec<Span<'a>> {
    vec![
        Span::styled("[", theme.dim),
        Span::styled(format!("{key}"), theme.key_chip),
        Span::raw(" "),
        Span::styled(label, style),
        Span::styled("]", theme.dim),
    ]
}

/// Build a deterministic animated spectrum from semantic icon ramp slots.
pub fn spectrum(
    width: usize,
    animation_frame: usize,
    theme: &Theme,
    icons: &Icons,
) -> Line<'static> {
    let spans = (0..width)
        .map(|column| {
            let index = (column.wrapping_mul(7) + animation_frame) % icons.spectrum_ramp.len();
            let style = if theme.truecolor {
                let ratio = column as f32 / width.max(1) as f32;
                let red = (34.0 + (217.0 - 34.0) * ratio) as u8;
                let green = (211.0 + (70.0 - 211.0) * ratio) as u8;
                let blue = (238.0 + (239.0 - 238.0) * ratio) as u8;
                Style::default().fg(Color::Rgb(red, green, blue))
            } else {
                theme.accent
            };
            Span::styled(icons.spectrum_ramp[index].to_string(), style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

/// Draw a standardized scrollbar only when content overflows its viewport.
pub fn scrollbar(frame: &mut Frame, area: Rect, content_len: usize, position: usize) {
    if content_len <= area.height as usize || area.is_empty() {
        return;
    }
    let mut state = ScrollbarState::new(content_len).position(position);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        area.inner(Margin {
            vertical: 0,
            horizontal: 0,
        }),
        &mut state,
    );
}

#[cfg(test)]
mod tests;
