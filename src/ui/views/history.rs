//! History view renderer.

use super::*;

use crate::app::state::HistoryViewMode;

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
    let title = format!("History ({total}) · {mode} · g toggles");
    let inner = section_panel(frame, area, &title, true, theme, icons);

    if total == 0 {
        frame.render_widget(
            Paragraph::new(Span::styled("No playback history", theme.dim)),
            inner,
        );
        return;
    }

    let inner = render_filter_bar(frame, inner, state, total, icons, theme);
    let lines = match state.history_view_mode {
        HistoryViewMode::Recent => {
            recent_rows(state, entries, &recent_indices, inner.width, icons, theme)
        }
        HistoryViewMode::Top => top_rows(state, top.unwrap_or_default(), inner.width, icons, theme),
    };
    state.list_state.select(Some(state.selected_index));
    state.list_hit_area = inner;
    frame.render_widget(Paragraph::new(lines), inner);
    scrollbar(frame, inner, total, state.list_state.offset());
}

fn recent_rows(
    state: &AppState,
    entries: &[crate::history::model::HistoryEntry],
    recent_indices: &[usize],
    width: u16,
    icons: &Icons,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let selected = state.resolve_index(state.selected_index);
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
            let title = format!(
                "{} — {}",
                sanitize_terminal_text(&entry.title),
                sanitize_terminal_text(&entry.artist)
            );
            let outcome = format!("{:?}", entry.outcome);
            let listened = format!(
                "{} · {}",
                outcome,
                format_time(entry.listened_seconds as f64)
            );
            Some(numbered_row(
                NumberedRow {
                    index: row,
                    title: &title,
                    right_columns: &[listened],
                    playing: false,
                    selected: selected == index,
                },
                width.saturating_sub(1) as usize,
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
) -> Vec<Line<'static>> {
    let selected = state.resolve_index(state.selected_index);
    visible_positions(state, stats.len())
        .into_iter()
        .filter_map(|index| {
            let stat = stats.get(index)?;
            let title = format!(
                "{} — {}",
                sanitize_terminal_text(&stat.entry.title),
                sanitize_terminal_text(&stat.entry.artist)
            );
            let counts = format!(
                "{} plays · {} tries · {} total",
                stat.completed_plays,
                stat.attempts,
                format_time(stat.total_listened_seconds as f64)
            );
            Some(numbered_row(
                NumberedRow {
                    index,
                    title: &title,
                    right_columns: &[counts],
                    playing: false,
                    selected: selected == index,
                },
                width.saturating_sub(1) as usize,
                theme,
                icons,
            ))
        })
        .collect()
}
