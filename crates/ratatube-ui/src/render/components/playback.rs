//! Shared bottom playback summary and final-window title transition.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::render::icons::{Icons, sanitize_terminal_text};
use crate::render::theme::Theme;
use crate::state::AppState;
use ratatube_domain::queue::RepeatMode;

/// Render the shared name, timeline, and playback-status rows.
pub fn playback_summary(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let Some(track) = &state.domain.current_track else {
        frame.render_widget(
            Paragraph::new(Span::styled("Nothing is playing", theme.dim)),
            area,
        );
        return;
    };
    if area.is_empty() {
        return;
    }

    let title = sanitize_terminal_text(&track.title);
    let channel = sanitize_terminal_text(&track.artist);
    let title_line = state
        .domain
        .track_transition
        .progress(Instant::now())
        .and_then(|progress| {
            state.domain.queue.effective_next().map(|next| {
                transition_line(
                    &title,
                    &sanitize_terminal_text(&next.title),
                    icons.chevron_l,
                    area.width as usize,
                    progress,
                    theme,
                )
            })
        })
        .unwrap_or_else(|| {
            Line::from(Span::styled(
                crate::render::widgets::truncate_middle(
                    &format!("{title} — {channel}"),
                    area.width as usize,
                ),
                theme.accent,
            ))
        });
    frame.render_widget(Paragraph::new(title_line), Rect { height: 1, ..area });
    render_timeline(frame, area, state, theme);
    render_status(frame, area, state, icons, theme);
}

fn transition_line(
    current: &str,
    next: &str,
    chevron: &str,
    width: usize,
    progress: f64,
    theme: &Theme,
) -> Line<'static> {
    if width == 0 {
        return Line::default();
    }
    let separator = format!(" {chevron} ");
    let separator_width = separator.width();
    if width <= separator_width {
        return Line::from(Span::styled(clip_end(current, width), theme.accent));
    }
    let content_width = width - separator_width;
    let final_next_width = (content_width / 2).max(1).min(next.width());
    let current_width = content_width.saturating_sub(final_next_width);
    let current = clip_end(current, current_width);
    let travel = width.saturating_sub(current.width());
    let offset = ((travel as f64) * (1.0 - progress.clamp(0.0, 1.0))).round() as usize;
    let available = width.saturating_sub(current.width() + offset);
    if available <= separator_width {
        return Line::from(Span::styled(clip_end(&current, width), theme.accent));
    }
    let next_width = (available - separator_width).min(final_next_width);
    Line::from(vec![
        Span::styled(current, theme.accent),
        Span::raw(" ".repeat(offset)),
        Span::styled(separator, theme.dim),
        Span::styled(clip_end(next, next_width), theme.value),
    ])
}

fn clip_end(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let budget = width.saturating_sub(1);
    let mut used = 0;
    let mut clipped = String::new();
    for character in text.chars() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > budget {
            break;
        }
        clipped.push(character);
        used += character_width;
    }
    clipped.push('…');
    clipped
}

fn render_timeline(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    if area.height < 2 {
        return;
    }
    let duration = state.domain.playback.duration_seconds.unwrap_or(0.0);
    let ratio = if duration > 0.0 {
        (state.domain.playback.position_seconds / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let width = area.width as usize;
    let filled = (ratio * width as f64).round() as usize;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                symbols::line::THICK.horizontal.repeat(filled),
                theme.gauge_filled,
            ),
            Span::styled(
                symbols::line::THICK
                    .horizontal
                    .repeat(width.saturating_sub(filled)),
                theme.border,
            ),
        ])),
        Rect {
            y: area.y + 1,
            height: 1,
            ..area
        },
    );
}

fn render_status(frame: &mut Frame, area: Rect, state: &AppState, icons: &Icons, theme: &Theme) {
    if area.height < 3 {
        return;
    }
    let row = Rect {
        y: area.y + 2,
        height: 1,
        ..area
    };
    let position = state.domain.playback.position_seconds;
    // Elapsed / total, matching the Quick Resume gauge; a right-hand value
    // after "/" always means the full track length.
    let time = format!(
        "{} / {}",
        crate::render::widgets::format_time(position),
        state
            .domain
            .playback
            .duration_seconds
            .map(crate::render::widgets::format_time)
            .unwrap_or_else(|| "--:--".into())
    );
    let time_width = time.width();
    frame.render_widget(Paragraph::new(Span::styled(time, theme.dim)), row);
    let shuffle = if state.domain.queue.shuffle {
        theme.accent
    } else {
        theme.dim
    };
    let repeat = if state.domain.queue.repeat == RepeatMode::Off {
        theme.dim
    } else {
        theme.accent
    };
    let gauge_width = 8usize;
    let filled = usize::from(state.domain.playback.volume.min(100)) * gauge_width / 100;
    let gauge = format!(
        "{}{}",
        icons.spectrum_ramp[3].repeat(filled),
        icons.spectrum_ramp[0].repeat(gauge_width - filled)
    );
    let controls = Line::from(vec![
        Span::styled(format!("{}  ", icons.shuffle), shuffle),
        Span::styled(format!("{}  ", icons.repeat), repeat),
        Span::styled(format!("{} ", icons.volume), theme.dim),
        Span::styled(gauge, theme.gauge_filled),
        Span::styled(format!(" {}%", state.domain.playback.volume), theme.dim),
    ]);
    let controls_width = controls.width();
    frame.render_widget(Paragraph::new(controls).alignment(Alignment::Right), row);
    render_level_meter(frame, row, time_width, controls_width, state, icons, theme);
}

/// Center the real-audio level meter in the gap between the time and the
/// controls; it appears only while sound is actually playing (bands decay
/// to nothing on pause, stop, and silence).
fn render_level_meter(
    frame: &mut Frame,
    row: Rect,
    time_width: usize,
    controls_width: usize,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let playing =
        state.domain.playback.status == ratatube_domain::playback::PlaybackStatus::Playing;
    let audible = state.ui.viz_bands.iter().any(|band| *band > 0.0);
    if !playing || !audible {
        return;
    }
    let meter_width = usize::from(super::visualizer::METER_WIDTH);
    let gap_start = time_width + 2;
    let gap_end = (row.width as usize).saturating_sub(controls_width + 2);
    if gap_end.saturating_sub(gap_start) < meter_width {
        return;
    }
    let meter_area = Rect {
        x: row.x + (gap_start + (gap_end - gap_start - meter_width) / 2) as u16,
        width: meter_width as u16,
        ..row
    };
    frame.render_widget(
        Paragraph::new(super::visualizer::meter_line(
            &state.ui.viz_bands,
            theme,
            icons,
        )),
        meter_area,
    );
}

#[cfg(test)]
mod tests;
