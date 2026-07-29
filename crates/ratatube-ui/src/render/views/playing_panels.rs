//! Lower panels for the responsive Playing view.

use super::*;
use crate::render::components::{
    EmptyState, NumberedRow, empty_state, numbered_row, scrollbar, section_panel,
};
use crate::state::PlayingPane;

pub fn render_up_next(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, "Up Next", false, theme, icons);
    let Some(position) = state.domain.queue.position else {
        empty_state(
            frame,
            inner,
            EmptyState {
                icon: icons.queue,
                headline: "Queue is empty",
                hints: &[("a", "queue from Search")],
            },
            theme,
        );
        return;
    };
    let upcoming = state
        .domain
        .queue
        .order
        .iter()
        .enumerate()
        .skip(position + 1);
    let total = state.domain.queue.order.len().saturating_sub(position + 1);
    let mut shown = 0usize;
    let lines = upcoming
        .take(inner.height as usize)
        .map(|(queue_position, &track_index)| {
            shown += 1;
            // An order entry that does not index a track means the queue broke
            // its own invariant. Render a placeholder rather than panicking the
            // render loop, and keep one row per order position so the row
            // numbering still matches the queue.
            let title = match state.domain.queue.tracks.get(track_index) {
                Some(track) => format!(
                    "{} — {}",
                    sanitize_terminal_text(&track.title),
                    sanitize_terminal_text(&track.artist)
                ),
                None => "Track unavailable".to_string(),
            };
            numbered_row(
                NumberedRow {
                    index: queue_position,
                    title: &title,
                    right_columns: &[],
                    playing: false,
                    selected: false,
                },
                inner.width as usize,
                theme,
                icons,
            )
        })
        .collect::<Vec<_>>();
    if shown == 0 {
        let message = if state.domain.queue.repeat == ratatube_domain::queue::RepeatMode::Queue {
            "Queue repeats from the top"
        } else {
            "Queue ends after this track"
        };
        frame.render_widget(Paragraph::new(Span::styled(message, theme.dim)), inner);
    } else {
        frame.render_widget(Paragraph::new(lines), inner);
        scrollbar(frame, inner, total, 0);
    }
}

pub fn render_description(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let chapters = state.chapters();
    if !chapters.is_empty() && !state.ui.now_playing_show_description {
        render_chapters(frame, area, state, chapters, icons, theme);
        return;
    }
    let inner = section_panel(frame, area, "Description", false, theme, icons);
    let description = state
        .domain
        .current_details
        .as_ref()
        .and_then(|details| details.description.as_deref())
        .unwrap_or("No description available.");
    let description = crate::render::icons::sanitize_multiline_text(description);
    let wrap_width = inner.width.max(1) as usize;
    let line_count = description
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(wrap_width))
        .sum::<usize>();
    frame.render_widget(
        Paragraph::new(description)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .style(theme.dim)
            .scroll((state.ui.now_playing_scroll, 0)),
        inner,
    );
    scrollbar(
        frame,
        inner,
        line_count,
        state.ui.now_playing_scroll as usize,
    );
}

fn render_chapters(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    chapters: &[ratatube_domain::media::Chapter],
    icons: &Icons,
    theme: &Theme,
) {
    let current = state.current_chapter_index();
    let title = format!("Tracklist ({})", chapters.len());
    let inner = section_panel(frame, area, &title, false, theme, icons);
    let visible = inner.height as usize;
    let offset = current
        .unwrap_or(0)
        .saturating_sub(visible.saturating_sub(1) / 2)
        .min(chapters.len().saturating_sub(visible));
    let lines = chapters
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(index, chapter)| {
            numbered_row(
                NumberedRow {
                    index,
                    title: &sanitize_terminal_text(&chapter.title),
                    right_columns: &[format_time(chapter.start_seconds)],
                    playing: Some(index) == current,
                    selected: false,
                },
                inner.width as usize,
                theme,
                icons,
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
    scrollbar(frame, inner, chapters.len(), offset);
}

pub fn render_queue(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let focused = state.ui.playing_pane == PlayingPane::Queue;
    let inner = section_panel(frame, area, "Queue", focused, theme, icons);
    let start = state
        .ui
        .selected_index
        .saturating_sub(inner.height as usize / 2);
    state.ui.list_hit_area = inner;
    // This pane windows rows manually; without the explicit offset mouse
    // clicks would map rows through the unrelated list-state offset.
    state.ui.list_hit_offset = Some(start);
    let lines = state
        .domain
        .queue
        .order
        .iter()
        .enumerate()
        .skip(start)
        .take(inner.height as usize)
        .map(|(position, &track_index)| {
            // One row per order position even when an entry is out of range:
            // this pane feeds `list_hit_offset`, so dropping a row would map
            // mouse clicks onto the wrong track.
            let title = match state.domain.queue.tracks.get(track_index) {
                Some(track) => sanitize_terminal_text(&track.title),
                None => "Track unavailable".to_string(),
            };
            numbered_row(
                NumberedRow {
                    index: position,
                    title: &title,
                    right_columns: &[],
                    playing: state.domain.queue.position == Some(position),
                    selected: focused && state.ui.selected_index == position,
                },
                inner.width as usize,
                theme,
                icons,
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
    scrollbar(
        frame,
        inner,
        state.domain.queue.order.len(),
        state.ui.selected_index,
    );
}
