//! Shared typed Activity panel renderer.

use chrono::Utc;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::history::activity::{ActivityKind, relative_time};
use crate::ui::components::{header_link, section_panel};
use crate::ui::icons::{Icons, sanitize_terminal_text};
use crate::ui::theme::Theme;

/// Render persisted activity with semantic icons and deterministic row shape.
pub fn render_activity_panel(
    frame: &mut Frame,
    area: Rect,
    state: &crate::app::state::AppState,
    focused: bool,
    clear_key: bool,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, "Activity", focused, theme, icons);
    if clear_key {
        frame.render_widget(
            Paragraph::new(Line::from(header_link("Clear", Some('c'), theme))).right_aligned(),
            Rect { height: 1, ..area },
        );
    }
    if state.domain.activity.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No activity yet", theme.dim)),
            inner,
        );
        return;
    }
    let now = Utc::now();
    let lines = state
        .domain
        .activity
        .entries()
        .iter()
        .take((inner.height as usize).div_ceil(2))
        .enumerate()
        .flat_map(|(index, event)| {
            let (icon, style) = match event.kind {
                ActivityKind::Played => (icons.play_btn, theme.success),
                ActivityKind::Queued => (icons.queue, theme.accent),
                ActivityKind::AddedToPlaylist => (icons.playlist, theme.accent_alt),
                ActivityKind::PlaylistImported => (icons.import, theme.warning),
            };
            [
                Line::from(vec![
                    Span::styled(format!("{icon} "), style),
                    Span::styled(
                        sanitize_terminal_text(&event.title),
                        if focused && state.ui.selected_index == index {
                            theme.selected
                        } else {
                            theme.base
                        },
                    ),
                ]),
                Line::from(Span::styled(
                    format!(
                        "  {}  {}",
                        sanitize_terminal_text(&event.detail),
                        relative_time(event.at, now)
                    ),
                    theme.dim,
                )),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}
