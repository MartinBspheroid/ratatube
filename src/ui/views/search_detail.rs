//! Selected-result detail panel and narrow modal.

use super::*;
use crate::ui::components::{key_value_rows, section_panel};

pub(super) fn render_selected(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    track: &crate::media::Track,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, "Selected", false, theme, icons);
    let detail_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(4)])
        .split(inner);
    let cached_for_track = state
        .ui
        .search_thumbnail_track_id
        .as_ref()
        .is_some_and(|track_id| track_id == &track.id);
    if cached_for_track && let Some(protocol) = state.ui.search_thumbnail.as_mut() {
        let image =
            ratatui_image::StatefulImage::<ratatui_image::protocol::StatefulProtocol>::new();
        frame.render_stateful_widget(image, detail_rows[0], protocol);
    } else {
        frame.render_widget(
            Paragraph::new(Span::styled(icons.music, theme.accent)).centered(),
            detail_rows[0],
        );
    }
    let duration = track
        .duration_seconds
        .map(|seconds| format_time(seconds as f64))
        .unwrap_or_else(|| "--:--".to_string());
    let pairs = vec![
        ("Duration".to_string(), duration),
        ("Channel".to_string(), sanitize_terminal_text(&track.artist)),
    ];
    let metadata = key_value_rows(&pairs, false, theme);
    frame.render_widget(
        Paragraph::new(
            vec![
                Line::from(Span::styled(
                    crate::ui::widgets::truncate_middle(
                        &sanitize_terminal_text(&track.title),
                        detail_rows[1].width as usize,
                    ),
                    theme.header,
                )),
                Line::from(Span::styled(
                    sanitize_terminal_text(&track.artist),
                    theme.accent,
                )),
            ]
            .into_iter()
            .chain(metadata)
            .collect::<Vec<_>>(),
        ),
        detail_rows[1],
    );
}

pub(super) fn render_overlay(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    track: &crate::media::Track,
    icons: &Icons,
    theme: &Theme,
) {
    use ratatui::widgets::Clear;
    let modal = crate::ui::centered_rect(area, 64, 14);
    frame.render_widget(Clear, modal);
    render_selected(frame, modal, state, track, icons, theme);
}
