//! Mini-player, spinners, and small formatting helpers.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crate::app::state::AppState;
use crate::ui::components::playback_summary;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;

/// Braille spinner frames, cycled by `spinner_frame`.
pub const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⦠", "⦧", "⦇", "⦏"];

/// Current spinner glyph for animations.
pub fn spinner(frame: usize) -> &'static str {
    SPINNER_FRAMES[frame % SPINNER_FRAMES.len()]
}

/// Render the mini player as a chrome strip: a top rule, then the shared
/// three-row playback summary (borders are reserved for overlays).
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
    frame.render_widget(
        Paragraph::new(Span::styled(
            icons.panel_rule.repeat(area.width as usize),
            theme.border,
        )),
        Rect { height: 1, ..area },
    );
    let inner = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };
    playback_summary(frame, inner, state, icons, theme);
}

/// Truncate `text` to `max_width` chars with a trailing ellipsis.
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

/// Truncate `text` with a middle ellipsis while retaining both ends.
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

/// Format seconds as m:ss, or h:mm:ss for long durations.
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

    use super::{format_time, truncate_end, truncate_middle};
    use crate::app::state::{AppState, View};
    use crate::config::IconMode;
    use crate::ui::header::{render_header, tab_hit_zones};
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
