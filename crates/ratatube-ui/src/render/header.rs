//! Header tabs, hit zones, and dependency status.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Tabs};
use unicode_width::UnicodeWidthStr;

use crate::render::icons::Icons;
use crate::render::layout::Breakpoint;
use crate::render::theme::Theme;
use crate::state::{AppState, View};

/// Tab titles, preserving a full active label on narrow terminals. Each
/// label leads with its `1`–`6` jump key so the shortcut is discoverable.
fn tab_titles(icons: &Icons, active: View, narrow: bool) -> Vec<(View, String)> {
    let ascii_mode = icons.playing == "[PLAY]";
    let views = [
        (View::Home, icons.home, "Home", "Hm"),
        (View::Search, icons.search, "Search", "Srch"),
        (View::Queue, icons.queue, "Queue", "Que"),
        (View::Playlists, icons.playlist, "Playlists", "Lists"),
        (View::History, icons.history, "History", "Hist"),
        (View::NowPlaying, icons.music, "Now Playing", "Play"),
    ];
    views
        .iter()
        .enumerate()
        .map(|(index, (view, icon, title, short))| {
            let key = index + 1;
            let text = if narrow {
                if *view == active {
                    format!("{key} {icon} {title}")
                } else if ascii_mode {
                    format!("{key} {short}")
                } else {
                    format!("{key} {icon}")
                }
            } else if ascii_mode {
                format!("{key} {title}")
            } else {
                format!("{key} {icon} {title}")
            };
            (*view, text)
        })
        .collect()
}

/// Clickable header x-ranges as `(view, start_col, end_col)`.
pub fn tab_hit_zones(icons: &Icons, active: View, narrow: bool) -> Vec<(View, u16, u16)> {
    let mut zones = Vec::new();
    let mut x = header_logo().width() as u16 + 2;
    for (view, title) in tab_titles(icons, active, narrow) {
        let width = title.width() as u16 + 2;
        zones.push((view, x, x + width));
        x += width + 1;
    }
    zones
}

fn header_logo() -> String {
    format!("ratatube v{}", env!("CARGO_PKG_VERSION"))
}

/// Render view tabs and dependency status when a dependency is unavailable.
pub(super) fn render_header(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let logo = header_logo();
    let breakpoint = Breakpoint::from_width(area.width);
    let show_volume = breakpoint >= Breakpoint::Medium;
    let show_queue = breakpoint >= Breakpoint::Wide;
    let dependency_problem = !state.domain.mpv_ready || !state.domain.yt_dlp_ready;
    let right_width = if dependency_problem && breakpoint >= Breakpoint::Medium {
        24
    } else {
        match (show_volume, show_queue) {
            (true, true) => 18,
            (true, false) => 5,
            _ => 0,
        }
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(logo.width() as u16 + 2),
            Constraint::Min(20),
            Constraint::Length(right_width),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("ratatube", theme.header),
            Span::styled(format!(" v{}", env!("CARGO_PKG_VERSION")), theme.dim),
        ])),
        chunks[0],
    );

    let narrow = breakpoint == Breakpoint::Narrow;
    let titles: Vec<Line> = tab_titles(icons, state.ui.view, narrow)
        .into_iter()
        .map(|(_, text)| Line::from(format!(" {text} ")))
        .collect();
    let selected = View::TABS
        .iter()
        .position(|view| *view == state.ui.view)
        .unwrap_or(0);
    frame.render_widget(
        Tabs::new(titles)
            .select(selected)
            .style(theme.dim)
            .highlight_style(theme.tab_active)
            .divider(Span::styled("│", theme.border))
            .padding("", ""),
        chunks[1],
    );

    let mut problems: Vec<Span> = Vec::new();
    if !state.domain.mpv_ready {
        problems.push(Span::styled(
            format!("{} mpv down   ", icons.error),
            theme.error,
        ));
    }
    if !state.domain.yt_dlp_ready {
        problems.push(Span::styled(
            format!("{} yt-dlp down", icons.error),
            theme.error,
        ));
    }
    if !dependency_problem && show_queue {
        problems.push(Span::styled(
            format!("  queue {}", state.domain.queue.tracks.len()),
            theme.dim,
        ));
    }
    if !problems.is_empty() && chunks[2].width > 0 {
        let status =
            Paragraph::new(Line::from(problems)).alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(status, chunks[2]);
    }
}
