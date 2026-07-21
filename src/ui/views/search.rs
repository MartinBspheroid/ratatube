//! Responsive Search view renderer.

use super::*;
use crate::ui::components::{scrollbar, section_panel};
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

    match state.search.clone() {
        SearchState::Idle => render_message(
            frame,
            rows[1],
            "Results",
            "Type a query and press Enter",
            icons.search,
            icons,
            theme,
        ),
        SearchState::Searching { query, .. } => render_message(
            frame,
            rows[1],
            "Results",
            &format!("Searching for \"{}\"...", sanitize_terminal_text(&query)),
            spinner(state.spinner_frame),
            icons,
            theme,
        ),
        SearchState::Failed { query, message } => render_message(
            frame,
            rows[1],
            &format!("Results for \"{}\" ", sanitize_terminal_text(&query)),
            &sanitize_terminal_text(&message),
            icons.error,
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

fn render_message(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    icon: &str,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, title, false, theme, icons);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(format!("{icon} "), theme.accent),
                Span::styled(message.to_string(), theme.dim),
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
    let inner = section_panel(frame, area, "Results", false, theme, icons);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!(
                "{} results for \"{}\"",
                tracks.len(),
                sanitize_terminal_text(query)
            ),
            theme.dim,
        )),
        rows[0],
    );
    if tracks.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No results — try another query", theme.dim)),
            rows[1],
        );
        return;
    }

    let table_rows = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let queued = state
                .queue
                .tracks
                .iter()
                .any(|queued| queued.id == track.id);
            let in_playlist = state.playlists.iter().any(|playlist| {
                playlist
                    .tracks
                    .iter()
                    .any(|playlist_track| playlist_track.id == track.id)
            });
            let marker = if state
                .current_track
                .as_ref()
                .is_some_and(|current| current.id == track.id)
            {
                icons.play_btn.to_string()
            } else {
                format!("{:02}", index + 1)
            };
            Row::new(vec![
                Cell::from(Span::styled(
                    if queued { icons.queue } else { "" },
                    theme.accent,
                )),
                Cell::from(Span::styled(
                    if in_playlist { icons.dot } else { "" },
                    theme.orange,
                )),
                Cell::from(marker),
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
    let table = Table::new(
        table_rows,
        [
            Constraint::Length(icons.queue.width() as u16 + 1),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Percentage(50),
            Constraint::Percentage(30),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(vec!["", "", "#", "TITLE", "CHANNEL", "DURATION"])
            .style(theme.dim)
            .bottom_margin(1),
    )
    .row_highlight_style(theme.selected);
    state.table_state.select(Some(state.selected_index));
    state.list_hit_area = Rect {
        y: rows[1].y + 2,
        height: rows[1].height.saturating_sub(2),
        ..rows[1]
    };
    frame.render_stateful_widget(table, rows[1], &mut state.table_state);
    scrollbar(frame, rows[1], tracks.len(), state.table_state.offset());
}
