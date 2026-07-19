//! Header tabs, hit zones, and dependency status.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Tabs};
use unicode_width::UnicodeWidthStr;

use crate::app::state::{AppState, View};
use crate::ui::components::spectrum;
use crate::ui::icons::Icons;
use crate::ui::layout::Breakpoint;
use crate::ui::theme::Theme;

/// Tab titles, preserving a full active label on narrow terminals.
pub fn tab_titles(icons: &Icons, active: View, narrow: bool) -> Vec<(View, String)> {
    let ascii_mode = icons.playing == "[PLAY]";
    let views = [
        (View::Home, icons.home, "Home", "Hm"),
        (View::Search, icons.search, "Search", "Srch"),
        (View::Queue, icons.queue, "Queue", "Que"),
        (View::Playlists, icons.playlist, "Playlists", "Lists"),
        (View::History, icons.history, "History", "Hist"),
        (View::NowPlaying, icons.music, "Playing", "Play"),
    ];
    views
        .iter()
        .map(|(view, icon, title, short)| {
            let text = if narrow {
                if *view == active {
                    format!("{icon} {title}")
                } else if ascii_mode {
                    (*short).to_string()
                } else {
                    (*icon).to_string()
                }
            } else if ascii_mode {
                (*title).to_string()
            } else {
                format!("{icon} {title}")
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
    format!("ytm v{}", env!("CARGO_PKG_VERSION"))
}

/// Render view tabs and dependency status when a dependency is unavailable.
pub fn render_header(
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
    let dependency_problem = !state.mpv_ready || !state.yt_dlp_ready;
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
            Span::styled("ytm", theme.header),
            Span::styled(format!(" v{}", env!("CARGO_PKG_VERSION")), theme.dim),
        ])),
        chunks[0],
    );

    let narrow = breakpoint == Breakpoint::Narrow;
    let titles: Vec<Line> = tab_titles(icons, state.view, narrow)
        .into_iter()
        .map(|(_, text)| Line::from(format!(" {text} ")))
        .collect();
    let selected = View::TABS
        .iter()
        .position(|view| *view == state.view)
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
    if !state.mpv_ready {
        problems.push(Span::styled(
            format!("{} mpv down   ", icons.error),
            theme.error,
        ));
    }
    if !state.yt_dlp_ready {
        problems.push(Span::styled(
            format!("{} yt-dlp down", icons.error),
            theme.error,
        ));
    }
    if !dependency_problem && show_volume {
        problems.extend(spectrum(4, state.spinner_frame, theme, icons).spans);
    }
    if !dependency_problem && show_queue {
        problems.push(Span::styled(
            format!("  queue {}", state.queue.tracks.len()),
            theme.dim,
        ));
    }
    if !problems.is_empty() && chunks[2].width > 0 {
        let status =
            Paragraph::new(Line::from(problems)).alignment(ratatui::layout::Alignment::Right);
        frame.render_widget(status, chunks[2]);
    }
}
