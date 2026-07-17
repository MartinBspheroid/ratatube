//! Reusable widgets: header tabs, now-playing panel, footer hints,
//! spinners, and small styled fragments.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, LineGauge, Paragraph, Tabs};

use crate::app::state::{AppState, View};
use crate::playback::PlaybackStatus;
use crate::ui::icons::Icons;
use crate::ui::icons::sanitize_terminal_text;
use crate::ui::theme::Theme;

/// Braille spinner frames, cycled by `spinner_frame` (PRD 17 loading states).
pub const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⦠", "⦧", "⦇", "⦏"];

/// Current spinner glyph for animations.
pub fn spinner(frame: usize) -> &'static str {
    SPINNER_FRAMES[frame % SPINNER_FRAMES.len()]
}

/// Tab titles per view; in ASCII mode the long icon fallbacks would crowd
/// the tabs, so titles alone carry the meaning (PRD 10.12).
pub fn tab_titles(icons: &Icons) -> Vec<(View, String)> {
    let ascii_mode = icons.playing == "[PLAY]";
    let views = [
        (View::Search, icons.search, "Search"),
        (View::Queue, icons.queue, "Queue"),
        (View::Playlists, icons.playlist, "Playlists"),
        (View::History, icons.history, "History"),
        (View::NowPlaying, icons.music, "Playing"),
    ];
    views
        .iter()
        .map(|(view, icon, title)| {
            let text = if ascii_mode {
                (*title).to_string()
            } else {
                format!("{icon} {title}")
            };
            (*view, text)
        })
        .collect()
}

/// Clickable x-ranges of the header tabs: (view, start_col, end_col).
/// Mirrors how the `Tabs` widget lays out titles with a 1-column divider.
pub fn tab_hit_zones(icons: &Icons) -> Vec<(View, u16, u16)> {
    let mut zones = Vec::new();
    let mut x = 0u16;
    for (view, title) in tab_titles(icons) {
        let width = title.chars().count() as u16;
        zones.push((view, x, x + width));
        // +1 for the "│" divider rendered between tabs.
        x += width + 1;
    }
    zones
}

/// Render the header: app badge, view tabs, and subprocess status.
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(30)])
        .split(area);

    let titles: Vec<Line> = tab_titles(icons)
        .into_iter()
        .map(|(_, text)| Line::from(text))
        .collect();
    let selected = View::TABS
        .iter()
        .position(|v| *v == state.view)
        .unwrap_or(0);
    let tabs = Tabs::new(titles)
        .select(selected)
        .style(theme.dim)
        .highlight_style(theme.tab_active)
        .divider(Span::styled("│", theme.border))
        .padding("", "");
    frame.render_widget(tabs, chunks[0]);

    let (mpv_style, mpv_text) = status_dot(state.mpv_ready, "mpv", theme);
    let (yt_style, yt_text) = status_dot(state.yt_dlp_ready, "yt-dlp", theme);
    let status = Paragraph::new(Line::from(vec![
        Span::styled("● ", mpv_style),
        Span::styled(format!("{mpv_text}  "), theme.dim),
        Span::styled("● ", yt_style),
        Span::styled(yt_text, theme.dim),
    ]))
    .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(status, chunks[1]);
}

fn status_dot(ready: bool, name: &str, theme: &Theme) -> (Style, String) {
    if ready {
        (theme.playing, format!("{name} ready"))
    } else {
        (theme.error, format!("{name} down"))
    }
}

use ratatui::style::Style;

/// Render the now-playing panel: state icon, title, progress gauge, volume.
pub fn render_now_playing(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    if area.height == 0 {
        return;
    }
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title(Span::styled(" Now Playing ", theme.header));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(track) = &state.current_track else {
        frame.render_widget(
            Paragraph::new(Span::styled("Nothing is playing", theme.dim)),
            inner,
        );
        return;
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    // Row 1: status icon, artist — title, modes + volume.
    let (status_icon, status_style) = match state.playback.status {
        PlaybackStatus::Playing => (icons.playing, theme.playing),
        PlaybackStatus::Paused => (icons.paused, theme.warning),
        _ => (icons.stopped, theme.dim),
    };
    let title = sanitize_terminal_text(&track.title);
    let artist = sanitize_terminal_text(&track.artist);

    let mut right = Vec::new();
    if state.queue.shuffle {
        right.push(Span::styled(format!("{} ", icons.shuffle), theme.accent));
    }
    match state.queue.repeat {
        crate::queue::RepeatMode::Track => {
            right.push(Span::styled(format!("{}1 ", icons.repeat), theme.accent));
        }
        crate::queue::RepeatMode::Queue => {
            right.push(Span::styled(format!("{} ", icons.repeat), theme.accent));
        }
        crate::queue::RepeatMode::Off => {}
    }
    let volume_icon = if state.playback.muted {
        icons.muted
    } else {
        icons.volume
    };
    right.push(Span::styled(
        format!("{volume_icon} {}%", state.playback.volume),
        theme.dim,
    ));
    let right_width: usize = right.iter().map(|s| s.content.chars().count()).sum();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),
            Constraint::Length(right_width as u16 + 1),
        ])
        .split(rows[0]);

    let info = Line::from(vec![
        Span::styled(format!("{status_icon} "), status_style),
        Span::styled(artist, theme.accent),
        Span::styled(" — ", theme.dim),
        Span::styled(title, theme.base),
    ]);
    frame.render_widget(Paragraph::new(info), cols[0]);
    frame.render_widget(
        Paragraph::new(Line::from(right)).alignment(ratatui::layout::Alignment::Right),
        cols[1],
    );

    // Row 2: progress gauge with elapsed/total.
    let position = state.playback.position_seconds;
    let duration = state.playback.duration_seconds.unwrap_or(0.0);
    let ratio = if duration > 0.0 {
        (position / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let label = format!(
        "{} / {}",
        format_time(position),
        state
            .playback
            .duration_seconds
            .map(format_time)
            .unwrap_or_else(|| "--:--".to_string())
    );
    let gauge = LineGauge::default()
        .filled_style(theme.gauge_filled)
        .filled_symbol(symbols::line::THICK.horizontal)
        .style(theme.border)
        .ratio(ratio)
        .label(Span::styled(label, theme.dim));
    frame.render_widget(gauge, rows[1]);
}

/// Render context-sensitive keyboard hints as styled chips (PRD 8).
pub fn render_footer(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let hints: &[(&str, &str)] = match state.view {
        View::Search => &[
            ("Enter", "play"),
            ("a", "queue"),
            ("A", "next"),
            ("/", "search"),
            ("?", "help"),
            ("q", "quit"),
        ],
        View::Queue => &[
            ("Enter", "play"),
            ("d", "remove"),
            ("c", "clear"),
            ("w", "save"),
            ("s", "shuffle"),
            ("r", "repeat"),
        ],
        View::Playlists => &[
            ("Enter", "open"),
            ("p", "play"),
            ("N", "new"),
            ("i", "import"),
            ("R", "rename"),
            ("x", "delete"),
        ],
        View::PlaylistDetail => &[("Enter", "play"), ("p", "play all"), ("Bksp", "back")],
        View::History => &[("Enter", "replay"), ("a", "queue"), ("c", "clear")],
        View::NowPlaying => &[
            ("Space", "pause"),
            ("h/l", "seek"),
            ("+/-", "volume"),
            ("n/b", "next/prev"),
        ],
        View::Help => &[("q", "quit"), ("1-5", "views")],
    };
    let mut spans = Vec::new();
    for (key, label) in hints {
        spans.push(Span::styled(format!(" {key} "), theme.key_chip));
        spans.push(Span::styled(format!("{label}  "), theme.dim));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Format seconds as m:ss (or h:mm:ss for long durations).
pub fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_time;

    #[test]
    fn formats_durations() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(65.4), "01:05");
        assert_eq!(format_time(3600.0), "1:00:00");
        assert_eq!(format_time(-5.0), "00:00");
    }
}
