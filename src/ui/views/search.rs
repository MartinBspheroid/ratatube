//! Responsive Search view renderer.

use super::*;
use crate::ui::components::{
    TrackRow, TrackTableLayout, header_row, marker_legend, scrollbar, section_panel, track_flags,
    track_row,
};
use crate::ui::layout::Breakpoint;
use crate::ui::views::search_detail::{render_overlay, render_selected};

pub(super) fn render_search(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(area);
    render_input(frame, rows[0], state, icons, theme);

    let results_focused = state.focus != Focus::SearchInput;
    match state.search.clone() {
        SearchState::Idle => render_message(
            frame,
            rows[1],
            ResultsMessage {
                title: "Results",
                message: "Type a query and press Enter",
                icon: icons.search,
                focused: results_focused,
            },
            icons,
            theme,
        ),
        SearchState::Searching { query, .. } => render_message(
            frame,
            rows[1],
            ResultsMessage {
                title: "Results",
                message: &format!("Searching for \"{}\"...", sanitize_terminal_text(&query)),
                icon: spinner(state.spinner_frame),
                focused: results_focused,
            },
            icons,
            theme,
        ),
        SearchState::Failed { query, message } => render_message(
            frame,
            rows[1],
            ResultsMessage {
                title: &format!("Results for \"{}\" ", sanitize_terminal_text(&query)),
                message: &sanitize_terminal_text(&message),
                icon: icons.error,
                focused: results_focused,
            },
            icons,
            theme,
        ),
        SearchState::Results { query, tracks } => {
            render_results(frame, rows[1], state, &query, &tracks, icons, theme);
        }
    }
}

fn render_input(frame: &mut Frame, area: Rect, state: &AppState, icons: &Icons, theme: &Theme) {
    let focused = state.focus == Focus::SearchInput;
    let inner = section_panel(frame, area, "Search", focused, theme, icons);
    let line = if state.search_input.is_empty() && !focused {
        Line::from(Span::styled(
            "Press / to search, or paste a YouTube URL...",
            theme.dim,
        ))
    } else {
        Line::from(vec![
            Span::raw(sanitize_terminal_text(&state.search_input)),
            Span::styled(if focused { icons.section_bar } else { "" }, theme.accent),
        ])
    };
    frame.render_widget(Paragraph::new(line), inner);
}

/// Content and focus state for the results placeholder panel.
struct ResultsMessage<'a> {
    title: &'a str,
    message: &'a str,
    icon: &'a str,
    focused: bool,
}

fn render_message(
    frame: &mut Frame,
    area: Rect,
    content: ResultsMessage<'_>,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, content.title, content.focused, theme, icons);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(format!("{} ", content.icon), theme.accent),
                Span::styled(content.message.to_string(), theme.dim),
            ]),
        ]),
        inner,
    );
}

fn render_results(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    query: &str,
    tracks: &[crate::media::Track],
    icons: &Icons,
    theme: &Theme,
) {
    let selected = tracks.get(state.selected_index).cloned();
    let breakpoint = Breakpoint::from_width(area.width);
    let columns = if breakpoint == Breakpoint::Narrow {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100), Constraint::Length(0)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(crate::ui::layout::LIST_DETAIL.map(Constraint::Percentage))
            .spacing(crate::ui::layout::PANE_GUTTER)
            .split(area)
    };
    render_result_table(frame, columns[0], state, query, tracks, icons, theme);
    if columns[1].width > 0
        && let Some(track) = &selected
    {
        render_selected(frame, columns[1], state, track, icons, theme);
    }
    if breakpoint == Breakpoint::Narrow
        && state.search_detail_open
        && let Some(track) = &selected
    {
        render_overlay(frame, area, state, track, icons, theme);
    }
}

fn render_result_table(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    query: &str,
    tracks: &[crate::media::Track],
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(
        frame,
        area,
        "Results",
        state.focus != Focus::SearchInput,
        theme,
        icons,
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);
    let mut meta = vec![Span::styled(
        format!(
            "{} results for \"{}\"",
            tracks.len(),
            sanitize_terminal_text(query)
        ),
        theme.dim,
    )];
    meta.extend(marker_legend(theme, icons));
    frame.render_widget(Paragraph::new(Line::from(meta)), rows[0]);
    if tracks.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No results — try another query", theme.dim)),
            rows[1],
        );
        return;
    }

    let layout = TrackTableLayout::new(rows[1].width, 8);
    let table_rows = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            track_row(
                &layout,
                TrackRow {
                    index,
                    title: sanitize_terminal_text(&track.title),
                    channel: sanitize_terminal_text(&track.artist),
                    right: track
                        .duration_seconds
                        .map(|seconds| format_time(seconds as f64))
                        .unwrap_or_else(|| "--:--".to_string()),
                    flags: track_flags(state, &track.id),
                },
                theme,
                icons,
            )
        })
        .collect::<Vec<_>>();
    let table = Table::new(table_rows, layout.constraints())
        .header(header_row("LENGTH", theme))
        .row_highlight_style(theme.selected)
        .highlight_symbol(icons.chevron_r);
    state.table_state.select(Some(state.selected_index));
    state.list_hit_area = Rect {
        y: rows[1].y + 2,
        height: rows[1].height.saturating_sub(2),
        ..rows[1]
    };
    frame.render_stateful_widget(table, rows[1], &mut state.table_state);
    scrollbar(frame, rows[1], tracks.len(), state.table_state.offset());
}
