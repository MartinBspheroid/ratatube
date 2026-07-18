//! Reusable widgets: header tabs, now-playing panel, footer hints,
//! spinners, and small styled fragments.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Tabs};
use unicode_width::UnicodeWidthStr;

use crate::app::state::{AppState, View};
use crate::ui::components::{playback_summary, spectrum};
use crate::ui::icons::Icons;
use crate::ui::layout::Breakpoint;
use crate::ui::theme::Theme;

/// Braille spinner frames, cycled by `spinner_frame` (PRD 17 loading states).
pub const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⦠", "⦧", "⦇", "⦏"];

/// Current spinner glyph for animations.
pub fn spinner(frame: usize) -> &'static str {
    SPINNER_FRAMES[frame % SPINNER_FRAMES.len()]
}

/// Tab titles per view. Wide terminals get "icon + text"; narrow terminals
/// keep a full active label while inactive tabs collapse to icons or short
/// ASCII labels (PRD 10.12, 20).
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

/// Clickable x-ranges of the header tabs: (view, start_col, end_col).
/// Mirrors how the `Tabs` widget lays out titles with 1-column padding on
/// each side and a 1-column divider.
pub fn tab_hit_zones(icons: &Icons, active: View, narrow: bool) -> Vec<(View, u16, u16)> {
    let mut zones = Vec::new();
    let mut x = header_logo().width() as u16 + 2;
    for (view, title) in tab_titles(icons, active, narrow) {
        // +2 for the padding spaces rendered around each title.
        let width = title.width() as u16 + 2;
        zones.push((view, x, x + width));
        // +1 for the "│" divider rendered between tabs.
        x += width + 1;
    }
    zones
}

fn header_logo() -> String {
    format!("ytm v{}", env!("CARGO_PKG_VERSION"))
}

/// Render the header: view tabs with breathing room, and dependency status
/// shown only when something is missing.
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
        // Keep padding inside the title so the active background covers it.
        .map(|(_, text)| Line::from(format!(" {text} ")))
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
    frame.render_widget(tabs, chunks[1]);

    // Dependency status is noise when everything works; only surface
    // problems (PRD 6: actionable when missing).
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

/// Render the three-row mini player.
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
        .border_style(theme.border);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    playback_summary(frame, inner, state, icons, theme);
}

/// Render context-sensitive keyboard hints as styled chips (PRD 8).
pub fn render_footer(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let hints: &[(&str, &str)] = match state.view {
        View::Home
            if state.home_section == crate::app::state::HomeSection::Resume
                && state.pending_resume.is_some() =>
        {
            &[
                ("Space", "resume"),
                ("h/l", "section"),
                ("/", "search"),
                ("?", "help"),
            ]
        }
        View::Home if state.home_section == crate::app::state::HomeSection::Recent => &[
            ("h/l", "section"),
            ("Enter", "play"),
            ("a", "queue"),
            ("P", "playlist"),
            ("/", "search"),
            ("?", "help"),
        ],
        View::Home if state.home_section == crate::app::state::HomeSection::Playlists => &[
            ("h/l", "section"),
            ("Enter", "play all"),
            ("p", "play all"),
            ("N", "new"),
            ("4", "view all"),
            ("?", "help"),
        ],
        View::Home => &[("h/l", "section"), ("/", "search"), ("?", "help")],
        View::Search
            if crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                == crate::ui::layout::Breakpoint::Narrow =>
        {
            &[
                ("Enter", "play"),
                ("a", "queue"),
                ("A", "next"),
                ("/", "search"),
                ("i", "details"),
                ("o", "browser"),
                ("?", "help"),
                ("q", "quit"),
            ]
        }
        View::Search => &[
            ("Enter", "play"),
            ("a", "queue"),
            ("A", "next"),
            ("P", "playlist"),
            ("o", "browser"),
            ("/", "search"),
            ("?", "help"),
        ],
        View::Queue => &[
            ("Enter", "play"),
            ("J/K", "move"),
            ("d", "remove"),
            ("u", "undo"),
            ("P", "playlist"),
            ("/", "filter"),
            ("w", "save"),
            ("s", "shuffle"),
        ],
        View::Playlists => &[
            ("Enter", "open"),
            ("p", "play"),
            ("N", "new"),
            ("i", "import"),
            ("I", "JSON"),
            ("R", "rename"),
            ("x", "delete"),
        ],
        View::PlaylistDetail => &[
            ("Enter", "play"),
            ("e", "edit details"),
            ("p", "play all"),
            ("J/K", "move"),
            ("d", "remove"),
            ("P", "copy to"),
            ("Bksp", "back"),
        ],
        View::History => match state.history_view_mode {
            crate::app::state::HistoryViewMode::Recent => &[
                ("Enter", "replay"),
                ("a", "queue"),
                ("P", "playlist"),
                ("/", "filter"),
                ("g", "top"),
                ("x", "delete"),
            ],
            crate::app::state::HistoryViewMode::Top => &[
                ("Enter", "replay"),
                ("a", "queue"),
                ("P", "playlist"),
                ("/", "filter"),
                ("g", "recent"),
            ],
        },
        View::NowPlaying
            if crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                == crate::ui::layout::Breakpoint::UltraWide
                && state.playing_pane == crate::app::state::PlayingPane::Queue =>
        {
            &[
                ("h/l", "info/queue"),
                ("j/k", "select"),
                ("Enter", "play"),
                ("Space", "pause"),
                ("+/-", "volume"),
            ]
        }
        View::NowPlaying
            if crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                == crate::ui::layout::Breakpoint::UltraWide =>
        {
            &[
                ("h/l", "info/queue"),
                ("j/k", "scroll"),
                ("Space", "pause"),
                ("./,", "chapter"),
                ("v", "pane"),
            ]
        }
        View::NowPlaying => &[
            ("Space", "pause"),
            ("h/l", "seek"),
            ("./,", "chapter"),
            ("+/-", "volume"),
            ("n/b", "next/prev"),
            ("v", "pane"),
        ],
        View::Help => &[("j/k", "scroll"), ("Esc/?", "return"), ("q", "quit")],
    };
    let mut spans = Vec::new();
    for (key, label) in hints {
        spans.push(Span::styled(format!(" {key} "), theme.key_chip));
        spans.push(Span::styled(format!("{label}  "), theme.dim));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Truncate `text` to `max_width` chars, replacing the overflow with a
/// trailing ellipsis. Used for URLs and secondary text.
pub fn truncate_end(text: &str, max_width: usize) -> String {
    let len = text.chars().count();
    if len <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let mut out: String = text.chars().take(max_width.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Truncate `text` to `max_width` chars with a middle ellipsis, keeping the
/// start and end visible. Used for titles, where the tail often carries
/// meaning ("… Essential Mix (15th May 2021)").
pub fn truncate_middle(text: &str, max_width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_width {
        return text.to_string();
    }
    if max_width < 5 {
        return truncate_end(text, max_width);
    }
    let keep = max_width - 1;
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(chars[chars.len() - tail..].iter());
    out
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
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::{format_time, render_header, tab_hit_zones, truncate_end, truncate_middle};
    use crate::app::state::{AppState, View};
    use crate::config::IconMode;
    use crate::ui::icons::icons_for;
    use crate::ui::theme::Theme;

    #[test]
    fn formats_durations() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(65.4), "01:05");
        assert_eq!(format_time(3600.0), "1:00:00");
        assert_eq!(format_time(-5.0), "00:00");
    }

    #[test]
    fn truncate_end_adds_ellipsis() {
        assert_eq!(truncate_end("short", 10), "short");
        assert_eq!(truncate_end("a longer string", 8), "a longe…");
        assert_eq!(truncate_end("abc", 0), "");
    }

    #[test]
    fn truncate_middle_keeps_both_ends() {
        assert_eq!(truncate_middle("short", 10), "short");
        let out = truncate_middle("Essential Mix (15th May 2021)", 20);
        assert_eq!(out.chars().count(), 20);
        assert!(out.starts_with("Essential"));
        assert!(out.ends_with("2021)"));
        // Below the useful minimum it degrades to end truncation.
        assert_eq!(truncate_middle("abcdefgh", 4), "abc…");
    }

    #[test]
    fn tab_hit_zones_include_logo_offset_and_active_width() {
        let icons = icons_for(IconMode::Ascii);
        let zones = tab_hit_zones(&icons, View::Search, true);
        assert!(zones[0].1 > 0, "logo occupies columns before tabs");
        assert!(zones.windows(2).all(|pair| pair[0].2 < pair[1].1));
        let search = zones
            .iter()
            .find(|(view, _, _)| *view == View::Search)
            .expect("search zone");
        let home = zones
            .iter()
            .find(|(view, _, _)| *view == View::Home)
            .expect("home zone");
        assert!(search.2 - search.1 > home.2 - home.1);
    }

    #[test]
    fn active_tab_highlight_includes_one_cell_of_side_padding() {
        let mut terminal = Terminal::new(TestBackend::new(100, 1)).expect("terminal");
        let mut state = AppState::new();
        state.mpv_ready = true;
        state.yt_dlp_ready = true;
        let icons = icons_for(IconMode::Ascii);
        let theme = Theme::from_truecolor(false);
        terminal
            .draw(|frame| render_header(frame, frame.area(), &state, &icons, &theme))
            .expect("draw");

        let (_, start, end) = tab_hit_zones(&icons, View::Home, false)[0];
        let buffer = terminal.backend().buffer();
        let active_background = theme.tab_active.bg.expect("active tab background");
        assert_eq!(
            buffer.cell((start, 0)).expect("left padding").bg,
            active_background
        );
        assert_eq!(
            buffer.cell((end - 1, 0)).expect("right padding").bg,
            active_background
        );
    }
}
