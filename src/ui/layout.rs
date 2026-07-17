//! Screen layout: header, main area, now-playing bar, footer (PRD 8).

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Documented minimum terminal dimensions (PRD 20). Below this the UI
/// degrades adaptively (panels hidden) but keeps working.
pub const MIN_COLS: u16 = 80;
pub const MIN_ROWS: u16 = 24;

/// Hard floor: below these dimensions only a warning can be shown.
pub const FLOOR_COLS: u16 = 40;
pub const FLOOR_ROWS: u16 = 10;

/// Rectangles for the four fixed screen areas.
#[derive(Debug, Clone, Copy)]
pub struct AppLayout {
    pub header: Rect,
    pub main: Rect,
    pub now_playing: Rect,
    pub footer: Rect,
}

impl AppLayout {
    /// Split `area` into the fixed regions. The now-playing panel collapses
    /// first on short terminals, the footer second (PRD 20: graceful
    /// degradation instead of failure).
    pub fn new(area: Rect, has_now_playing: bool, show_footer: bool) -> Self {
        let now_playing_height = if has_now_playing && area.height >= 14 {
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
pub fn is_compact(area: Rect) -> bool {
    area.width < FLOOR_COLS || area.height < FLOOR_ROWS
}

/// Whether the terminal is below the documented minimum; the UI renders
/// adaptively but should tell the user (PRD 20).
pub fn is_small(area: Rect) -> bool {
    area.width < MIN_COLS || area.height < MIN_ROWS
}
