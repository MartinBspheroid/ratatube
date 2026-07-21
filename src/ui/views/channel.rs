//! Responsive dedicated channel browser.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

use crate::app::state::AppState;
use crate::ui::components::{scrollbar, section_panel};
use crate::ui::icons::{Icons, sanitize_terminal_text};
use crate::ui::layout::Breakpoint;
use crate::ui::theme::Theme;
use crate::ui::widgets::{format_time, spinner};

pub(super) fn render_channel(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let Some(channel) = state.channel.clone() else {
        frame.render_widget(Paragraph::new("Channel is unavailable"), area);
        return;
    };
    let wide = Breakpoint::from_width(area.width) != Breakpoint::Narrow;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if wide {
            crate::ui::layout::LIST_DETAIL.map(Constraint::Percentage)
        } else {
            [Constraint::Percentage(100), Constraint::Length(0)]
        })
        .spacing(if wide {
            crate::ui::layout::PANE_GUTTER
        } else {
            0
        })
        .split(area);
    render_table(frame, columns[0], state, &channel, icons, theme);
    if columns[1].width > 0 {
        render_preview(frame, columns[1], state, &channel, icons, theme);
    }
}

fn render_table(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    channel: &crate::app::channel::ChannelState,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(
        frame,
        area,
        &format!("{} · newest first", sanitize_terminal_text(&channel.name)),
        true,
        theme,
        icons,
    );
    let mut rows = channel
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            Row::new(vec![
                Cell::from(format!("{:02}", index + 1)),
                Cell::from(sanitize_terminal_text(&track.title)),
                Cell::from(sanitize_terminal_text(&track.artist)),
                Cell::from(
                    track
                        .duration_seconds
                        .map(|seconds| format_time(seconds as f64))
                        .unwrap_or_else(|| "--:--".to_string()),
                ),
            ])
        })
        .collect::<Vec<_>>();
    if channel.loading {
        rows.push(Row::new(vec![
            Cell::from(""),
            Cell::from(format!("{} Loading…", spinner(state.spinner_frame))),
            Cell::from(""),
            Cell::from(""),
        ]));
    } else if !channel.exhausted {
        let label = if channel.error.is_some() {
            "Retry…"
        } else {
            "Load more…"
        };
        let detail = channel
            .error
            .as_deref()
            .map(sanitize_terminal_text)
            .unwrap_or_default();
        rows.push(Row::new(vec![
            Cell::from(""),
            Cell::from(label),
            Cell::from(detail),
            Cell::from(""),
        ]));
    }
    if rows.is_empty() {
        let message = channel
            .error
            .as_deref()
            .unwrap_or("This channel has no public videos");
        frame.render_widget(Paragraph::new(sanitize_terminal_text(message)), inner);
        return;
    }
    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Percentage(55),
            Constraint::Percentage(30),
            Constraint::Length(8),
        ],
    )
    .header(Row::new(["#", "TITLE", "CHANNEL", "DURATION"]).style(theme.dim))
    .row_highlight_style(theme.selected);
    state.table_state.select(Some(state.selected_index));
    state.list_hit_area = inner;
    frame.render_stateful_widget(table, inner, &mut state.table_state);
    scrollbar(
        frame,
        inner,
        channel.tracks.len(),
        state.table_state.offset(),
    );
}

fn render_preview(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    channel: &crate::app::channel::ChannelState,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, "Selection", false, theme, icons);
    let lines = match channel.tracks.get(state.selected_index) {
        Some(track) => vec![
            Line::from(Span::styled(
                sanitize_terminal_text(&track.title),
                theme.accent,
            )),
            Line::from(Span::styled(
                sanitize_terminal_text(&track.artist),
                theme.base,
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Enter play · c actions · Backspace back",
                theme.dim,
            )),
        ],
        None if channel.loading => vec![Line::from(Span::styled("Loading videos…", theme.dim))],
        None if channel.error.is_some() => vec![
            Line::from(Span::styled("Page failed", theme.error)),
            Line::from(Span::styled("Select Retry and press Enter", theme.dim)),
        ],
        None => vec![Line::from(Span::styled(
            "Load the next 30 videos",
            theme.dim,
        ))],
    };
    frame.render_widget(Paragraph::new(lines), inner);
}
