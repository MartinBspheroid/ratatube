//! Screen layout: header, main area, now-playing bar, footer (PRD 8).

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Documented minimum terminal dimensions (PRD 20). Below this the UI
/// degrades adaptively (panels hidden) but keeps working.
pub(super) const MIN_COLS: u16 = 80;
pub(super) const MIN_ROWS: u16 = 24;

/// Hard floor: below these dimensions only a warning can be shown.
pub(super) const FLOOR_COLS: u16 = 40;
pub(super) const FLOOR_ROWS: u16 = 10;

/// One-cell gap between adjacent panes so separation survives terminals
/// where `panel_bg` is a no-op (non-truecolor).
pub(super) const PANE_GUTTER: u16 = 1;

/// Shared list/detail split for browse views (Search, Channel).
pub(super) const LIST_DETAIL: [u16; 2] = [65, 35];

/// Master/detail split for the Playlists overview.
pub(super) const PLAYLIST_MASTER: [u16; 2] = [38, 62];

/// Minimum pane width for the Home playlists card grid.
pub(super) const HOME_GRID_MIN_WIDTH: u16 = 68;

/// Minimum editor width before the playlist inspector pane appears.
pub(super) const INSPECTOR_MIN_WIDTH: u16 = 88;

/// Minimum Quick Resume pane width before thumbnail art appears.
pub(super) const RESUME_ART_MIN_WIDTH: u16 = 36;

/// Responsive layout band selected only from terminal width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Breakpoint {
    Narrow,
    Medium,
    Wide,
    UltraWide,
}

impl Breakpoint {
    /// Classify terminal columns using the provisional mock-layout thresholds.
    pub const fn from_width(width: u16) -> Self {
        match width {
            0..100 => Self::Narrow,
            100..140 => Self::Medium,
            140..170 => Self::Wide,
            _ => Self::UltraWide,
        }
    }
}

/// Rectangles for the four fixed screen areas.
#[derive(Debug, Clone, Copy)]
pub struct AppLayout {
    pub(super) header: Rect,
    pub(super) main: Rect,
    pub now_playing: Rect,
    pub(super) footer: Rect,
}

impl AppLayout {
    /// Split `area` into the fixed regions. The now-playing panel collapses
    /// first on short terminals, the footer second (PRD 20: graceful
    /// degradation instead of failure).
    pub fn new(area: Rect, has_now_playing: bool, show_footer: bool) -> Self {
        let now_playing_height = if has_now_playing && area.height >= 16 {
            // Top rule + name + full-width timeline + status row.
            4
        } else {
            0
        };
        let footer_height = u16::from(show_footer && area.height >= 18);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(now_playing_height),
                Constraint::Length(footer_height),
            ])
            .split(area);
        Self {
            header: chunks[0],
            main: chunks[1],
            now_playing: chunks[2],
            footer: chunks[3],
        }
    }
}

/// Whether the terminal is below the hard floor and cannot render the UI.
pub(super) fn is_compact(area: Rect) -> bool {
    area.width < FLOOR_COLS || area.height < FLOOR_ROWS
}

/// Whether the terminal is below the documented minimum; the UI renders
/// adaptively but should tell the user (PRD 20).
pub(super) fn is_small(area: Rect) -> bool {
    area.width < MIN_COLS || area.height < MIN_ROWS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoint_boundaries_are_explicit() {
        assert_eq!(Breakpoint::from_width(99), Breakpoint::Narrow);
        assert_eq!(Breakpoint::from_width(100), Breakpoint::Medium);
        assert_eq!(Breakpoint::from_width(139), Breakpoint::Medium);
        assert_eq!(Breakpoint::from_width(140), Breakpoint::Wide);
        assert_eq!(Breakpoint::from_width(169), Breakpoint::Wide);
        assert_eq!(Breakpoint::from_width(170), Breakpoint::UltraWide);
    }

    #[test]
    fn mini_player_reserves_three_content_rows() {
        let layout = AppLayout::new(Rect::new(0, 0, 120, 30), true, true);
        assert_eq!(layout.now_playing.height, 4);
        assert_eq!(layout.footer.height, 1);
        assert_eq!(layout.main.height, 24);
    }

    #[test]
    fn mini_player_keeps_all_three_rows_below_thirty() {
        let layout = AppLayout::new(Rect::new(0, 0, 80, 29), true, true);
        assert_eq!(layout.now_playing.height, 4);
        assert_eq!(layout.footer.height, 1);
    }

    #[test]
    fn degradation_gates_remain_explicit() {
        assert_eq!(
            AppLayout::new(Rect::new(0, 0, 80, 15), true, true)
                .now_playing
                .height,
            0
        );
        assert_eq!(
            AppLayout::new(Rect::new(0, 0, 80, 16), true, true)
                .now_playing
                .height,
            4
        );
        assert_eq!(
            AppLayout::new(Rect::new(0, 0, 80, 17), true, true)
                .footer
                .height,
            0
        );
        assert_eq!(
            AppLayout::new(Rect::new(0, 0, 80, 18), true, true)
                .footer
                .height,
            1
        );
    }
}
