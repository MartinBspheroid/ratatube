//! History view renderer.

use super::*;

use crate::app::state::HistoryViewMode;
use crate::ui::components::{
    EmptyState, TrackRow, TrackTableLayout, empty_state, header_row, track_flags, track_row,
};

pub(super) fn render_history(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    history: Option<&HistoryService>,
    icons: &Icons,
    theme: &Theme,
) {
    let entries = history.map(HistoryService::entries).unwrap_or(&[]);
    let recent_indices = history
        .map(HistoryService::recent_unique_indices)
        .unwrap_or_default();
    let top = (state.history_view_mode == HistoryViewMode::Top)
        .then(|| history.map(HistoryService::aggregate).unwrap_or_default());
    let total = top.map_or(recent_indices.len(), <[_]>::len);
    let mode = match state.history_view_mode {
        HistoryViewMode::Recent => "recent",
        HistoryViewMode::Top => "top",
    };
    let title = format!("History ({total}) · {mode}");
    let inner = section_panel(frame, area, &title, true, theme, icons);

    if total == 0 {
        empty_state(
            frame,
            inner,
            EmptyState {
                icon: icons.history,
                headline: "No playback history",
                hints: &[("/", "search"), ("Enter", "play a result")],
            },
            theme,
        );
        return;
    }

    let inner = render_filter_bar(frame, inner, state, total, icons, theme);
    let (right_header, rows) = match state.history_view_mode {
        HistoryViewMode::Recent => (
            "LISTENED",
            recent_rows(state, entries, &recent_indices, inner.width, icons, theme),
        ),
        HistoryViewMode::Top => (
            "PLAYS",
            top_rows(state, top.unwrap_or_default(), inner.width, icons, theme),
        ),
    };
    let visible_total = rows.len();
    let table = Table::new(rows, history_layout(state, inner.width).constraints())
        .header(header_row(right_header, theme))
        .row_highlight_style(theme.selected)
        .highlight_symbol(icons.chevron_r);
    state.table_state.select(Some(state.selected_index));
    state.list_hit_area = Rect {
        y: inner.y + 2,
        height: inner.height.saturating_sub(2),
        ..inner
    };
    frame.render_stateful_widget(table, inner, &mut state.table_state);
    scrollbar(frame, inner, visible_total, state.table_state.offset());
}

/// Human wording for a playback outcome (never Debug formatting).
fn outcome_label(outcome: crate::history::model::PlaybackOutcome) -> &'static str {
    match outcome {
        crate::history::model::PlaybackOutcome::Completed => "finished",
        crate::history::model::PlaybackOutcome::Skipped => "skipped",
        crate::history::model::PlaybackOutcome::Failed => "failed",
        crate::history::model::PlaybackOutcome::Stopped => "stopped",
    }
}

/// Recent rows carry `outcome · time`; Top rows carry play statistics.
fn history_layout(state: &AppState, width: u16) -> TrackTableLayout {
    let right_width = match state.history_view_mode {
        HistoryViewMode::Recent => 20,
        HistoryViewMode::Top => 34,
    };
    TrackTableLayout::new(width, right_width)
}

fn recent_rows(
    state: &AppState,
    entries: &[crate::history::model::HistoryEntry],
    recent_indices: &[usize],
    width: u16,
    icons: &Icons,
    theme: &Theme,
) -> Vec<ratatui::widgets::Row<'static>> {
    let layout = history_layout(state, width);
    let indices = state
        .visible_indices
        .as_deref()
        .unwrap_or(recent_indices)
        .to_vec();
    indices
        .into_iter()
        .enumerate()
        .filter_map(|(row, index)| {
            let entry = entries.get(index)?;
            let listened = format!(
                "{} · {}",
                outcome_label(entry.outcome),
                format_time(entry.listened_seconds as f64)
            );
            Some(track_row(
                &layout,
                TrackRow {
                    index: row,
                    title: sanitize_terminal_text(&entry.title),
                    channel: sanitize_terminal_text(&entry.artist),
                    right: listened,
                    flags: track_flags(state, &entry.track_id),
                },
                theme,
                icons,
            ))
        })
        .collect()
}

fn top_rows(
    state: &AppState,
    stats: &[crate::history::service::TrackStats],
    width: u16,
    icons: &Icons,
    theme: &Theme,
) -> Vec<ratatui::widgets::Row<'static>> {
    let layout = history_layout(state, width);
    visible_positions(state, stats.len())
        .into_iter()
        .filter_map(|index| {
            let stat = stats.get(index)?;
            let counts = format!(
                "{} plays · {} tries · {} total",
                stat.completed_plays,
                stat.attempts,
                format_time(stat.total_listened_seconds as f64)
            );
            Some(track_row(
                &layout,
                TrackRow {
                    index,
                    title: sanitize_terminal_text(&stat.entry.title),
                    channel: sanitize_terminal_text(&stat.entry.artist),
                    right: counts,
                    flags: track_flags(state, &stat.entry.track_id),
                },
                theme,
                icons,
            ))
        })
        .collect()
}
