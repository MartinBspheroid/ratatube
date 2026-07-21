//! Queue view renderer.

use super::*;
use crate::ui::components::{
    TrackFlags, TrackRow, TrackTableLayout, header_row, track_flags, track_row,
};

pub(super) fn render_queue(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let total = state.queue.order.len();
    let modes = queue_modes(state);
    let title = format!("Queue ({total}){modes}");
    let inner = section_panel(frame, area, &title, true, theme, icons);
    let inner = render_filter_bar(frame, inner, state, total, icons, theme);

    if total == 0 {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("Queue is empty", theme.dim)),
                Line::from(""),
                Line::from(Span::styled(
                    "Search for something and press a to add it",
                    theme.dim,
                )),
            ]),
            inner,
        );
        return;
    }

    let layout = TrackTableLayout::new(inner.width, 8);
    let rows = visible_positions(state, total)
        .into_iter()
        .filter_map(|position| {
            let track_index = *state.queue.order.get(position)?;
            let track = state.queue.tracks.get(track_index)?;
            Some(track_row(
                &layout,
                TrackRow {
                    index: position,
                    title: sanitize_terminal_text(&track.title),
                    channel: sanitize_terminal_text(&track.artist),
                    right: track
                        .duration_seconds
                        .map(|seconds| format_time(seconds as f64))
                        .unwrap_or_else(|| "--:--".to_string()),
                    // Every row here is queued by definition; the marker
                    // column carries the cross-tab states that add signal.
                    flags: TrackFlags {
                        playing: state.queue.position == Some(position),
                        queued: false,
                        in_playlist: track_flags(state, &track.id).in_playlist,
                    },
                },
                theme,
                icons,
            ))
        })
        .collect::<Vec<_>>();
    let visible_total = rows.len();
    let table = Table::new(rows, layout.constraints())
        .header(header_row("LENGTH", theme))
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

fn queue_modes(state: &AppState) -> String {
    let mut modes = Vec::new();
    if state.queue.shuffle {
        modes.push("shuffle");
    }
    match state.queue.repeat {
        crate::queue::RepeatMode::Track => modes.push("repeat track"),
        crate::queue::RepeatMode::Queue => modes.push("repeat queue"),
        crate::queue::RepeatMode::Off => {}
    }
    if modes.is_empty() {
        String::new()
    } else {
        format!(" · {}", modes.join(" · "))
    }
}
