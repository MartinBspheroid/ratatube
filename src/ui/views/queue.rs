//! Queue view renderer.

use super::*;

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

    let selected = state.resolve_index(state.selected_index);
    let lines = visible_positions(state, total)
        .into_iter()
        .filter_map(|position| {
            let track_index = *state.queue.order.get(position)?;
            let track = state.queue.tracks.get(track_index)?;
            let duration = track
                .duration_seconds
                .map(|seconds| format_time(seconds as f64))
                .unwrap_or_else(|| "--:--".to_string());
            let title = format!(
                "{} — {}",
                sanitize_terminal_text(&track.title),
                sanitize_terminal_text(&track.artist)
            );
            Some(numbered_row(
                NumberedRow {
                    index: position,
                    title: &title,
                    right_columns: &[duration],
                    playing: state.queue.position == Some(position),
                    selected: selected == position,
                },
                inner.width.saturating_sub(1) as usize,
                theme,
                icons,
            ))
        })
        .collect::<Vec<_>>();
    state.list_state.select(Some(state.selected_index));
    state.list_hit_area = inner;
    frame.render_widget(Paragraph::new(lines), inner);
    scrollbar(frame, inner, total, state.list_state.offset());
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
