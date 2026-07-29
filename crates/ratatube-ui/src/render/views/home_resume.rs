//! Quick Resume panel for the Home dashboard.

use super::*;
use crate::render::views::home_panels::panel;
use crate::state::{HomeSection, PendingResume};

pub(super) fn render_resume(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    history: Option<&HistoryLog>,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = panel(
        frame,
        area,
        "Quick Resume",
        state.ui.home_section == HomeSection::Resume,
        None,
        icons,
        theme,
    );
    let Some(pending) = state.domain.pending_resume.clone() else {
        frame.render_widget(
            Paragraph::new(Span::styled("Nothing to resume yet", theme.dim)),
            inner,
        );
        return;
    };
    // The whole card is one clickable item.
    state.ui.home_hit_zones.push(crate::state::HomeHitZone {
        section: HomeSection::Resume,
        index: 0,
        area: inner,
    });
    let show_art = inner.width >= crate::render::layout::RESUME_ART_MIN_WIDTH;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if show_art {
            [Constraint::Length(12), Constraint::Min(12)]
        } else {
            [Constraint::Length(0), Constraint::Min(12)]
        })
        .split(inner);
    if show_art {
        render_resume_art(frame, columns[0], state, &pending, icons, theme);
    }
    let info = columns[1];
    let last_played = history
        .and_then(|service| {
            service
                .entries()
                .iter()
                .find(|entry| entry.track_id == pending.track.id)
        })
        .map(|entry| {
            entry
                .played_at
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "previous session".to_string());
    let duration = pending.track.duration_seconds.unwrap_or(0) as f64;
    let ratio = if duration > 0.0 {
        (pending.position_seconds / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let lines = vec![
        Line::from(Span::styled(
            crate::render::widgets::truncate_middle(
                &sanitize_terminal_text(&pending.track.title),
                info.width as usize,
            ),
            theme.header,
        )),
        Line::from(Span::styled(
            sanitize_terminal_text(&pending.track.artist),
            theme.accent,
        )),
        Line::from(Span::styled(
            format!("Last played {last_played}"),
            theme.dim,
        )),
    ];
    frame.render_widget(Paragraph::new(lines), info);
    if info.height >= 4 {
        frame.render_widget(
            ratatui::widgets::LineGauge::default()
                .filled_style(theme.gauge_filled)
                .style(theme.border)
                .ratio(ratio)
                .label(Span::styled(
                    format!(
                        "{} / {}",
                        format_time(pending.position_seconds),
                        pending
                            .track
                            .duration_seconds
                            .map(|seconds| format_time(seconds as f64))
                            .unwrap_or_else(|| "--:--".to_string())
                    ),
                    theme.base,
                )),
            Rect {
                y: info.y + 3,
                height: 1,
                ..info
            },
        );
    }
}

fn render_resume_art(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    pending: &PendingResume,
    icons: &Icons,
    theme: &Theme,
) {
    let cached_for_track = state
        .domain
        .current_track
        .as_ref()
        .map(|track| track.id.as_str())
        == Some(pending.track.id.as_str());
    if cached_for_track && let Some(protocol) = state.ui.thumbnail.as_mut() {
        let image =
            ratatui_image::StatefulImage::<ratatui_image::protocol::StatefulProtocol>::new();
        frame.render_stateful_widget(image, area, protocol);
    } else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(icons.music, theme.accent)).centered(),
            ]),
            area,
        );
    }
}
