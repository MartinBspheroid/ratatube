//! Shared track table: one column vocabulary (`TITLE / CHANNEL / LENGTH`),
//! selection style, and marker language for every track list in the app.

use ratatui::layout::Constraint;
use ratatui::text::Span;
use ratatui::widgets::{Cell, Row};
use unicode_width::UnicodeWidthStr;

use crate::app::state::AppState;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use crate::ui::widgets::truncate_end;

/// Width of the leading marker column (glyph + gap).
const MARKER_WIDTH: u16 = 2;
/// Width of the two-digit index column.
const INDEX_WIDTH: u16 = 3;
/// Inter-column spacing ratatui inserts between the five columns.
const COLUMN_SPACING: u16 = 4;
/// Cell reserved for the selection highlight symbol.
const HIGHLIGHT_WIDTH: u16 = 1;

/// Cross-tab track state shown in the marker column.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrackFlags {
    pub playing: bool,
    pub queued: bool,
    pub in_playlist: bool,
}

/// Resolve the marker flags for a track ID against global state.
pub fn track_flags(state: &AppState, track_id: &str) -> TrackFlags {
    TrackFlags {
        playing: state
            .domain
            .current_track
            .as_ref()
            .is_some_and(|current| current.id == track_id),
        queued: state
            .domain
            .queue
            .tracks
            .iter()
            .any(|track| track.id == track_id),
        in_playlist: state.domain.playlists.iter().any(|playlist| {
            playlist
                .tracks
                .iter()
                .any(|playlist_track| playlist_track.id == track_id)
        }),
    }
}

/// One row of the shared track table.
pub struct TrackRow {
    pub index: usize,
    pub title: String,
    pub channel: String,
    pub right: String,
    pub flags: TrackFlags,
}

/// Column budget for one table instance, derived from the pane width.
#[derive(Debug, Clone, Copy)]
pub struct TrackTableLayout {
    title_width: u16,
    channel_width: u16,
    right_width: u16,
}

impl TrackTableLayout {
    /// Split `width` into the five standard columns; `right_width` sizes the
    /// trailing column (8 fits `LENGTH`, wider fits history statistics).
    pub fn new(width: u16, right_width: u16) -> Self {
        let fixed = MARKER_WIDTH + INDEX_WIDTH + COLUMN_SPACING + HIGHLIGHT_WIDTH + right_width;
        let flexible = width.saturating_sub(fixed);
        let title_width = flexible * 3 / 5;
        Self {
            title_width,
            channel_width: flexible.saturating_sub(title_width),
            right_width,
        }
    }

    /// Ratatui constraints matching the pre-truncated cell contents.
    pub fn constraints(&self) -> [Constraint; 5] {
        [
            Constraint::Length(MARKER_WIDTH),
            Constraint::Length(INDEX_WIDTH),
            Constraint::Length(self.title_width),
            Constraint::Length(self.channel_width),
            Constraint::Length(self.right_width),
        ]
    }
}

/// Standard dim column header; `right_header` is `LENGTH` unless a view has
/// a domain-specific trailing column (for example `LISTENED`).
pub fn header_row<'a>(right_header: &'a str, theme: &Theme) -> Row<'a> {
    Row::new(["", "#", "TITLE", "CHANNEL", right_header])
        .style(theme.label)
        .bottom_margin(1)
}

/// Build one standardized row with ellipsis truncation and marker glyph.
pub fn track_row(
    layout: &TrackTableLayout,
    row: TrackRow,
    theme: &Theme,
    icons: &Icons,
) -> Row<'static> {
    let (marker, marker_style) = marker(row.flags, theme, icons);
    Row::new(vec![
        Cell::from(Span::styled(marker.to_string(), marker_style)),
        Cell::from(Span::styled(format!("{:02}", row.index + 1), theme.dim)),
        Cell::from(ellipsized(&row.title, layout.title_width)),
        Cell::from(ellipsized(&row.channel, layout.channel_width)),
        Cell::from(Span::styled(
            ellipsized(&row.right, layout.right_width),
            theme.dim,
        )),
    ])
}

/// Build a full-width message row (for example `Load more…` / `Retry…`).
pub fn message_row(label: String, detail: String, theme: &Theme) -> Row<'static> {
    Row::new(vec![
        Cell::from(""),
        Cell::from(""),
        Cell::from(Span::styled(label, theme.link)),
        Cell::from(Span::styled(detail, theme.dim)),
        Cell::from(""),
    ])
}

/// Marker-legend spans for a results meta line or help text.
pub fn marker_legend(theme: &Theme, icons: &Icons) -> Vec<Span<'static>> {
    vec![
        Span::styled(format!("  ·  {} ", icons.play_btn), theme.success),
        Span::styled("playing  ", theme.dim),
        Span::styled(format!("{} ", icons.marker_queued), theme.accent),
        Span::styled("queued  ", theme.dim),
        Span::styled(format!("{} ", icons.dot), theme.orange),
        Span::styled("in playlist", theme.dim),
    ]
}

fn marker(
    flags: TrackFlags,
    theme: &Theme,
    icons: &Icons,
) -> (&'static str, ratatui::style::Style) {
    if flags.playing {
        (icons.play_btn, theme.success)
    } else if flags.queued {
        (icons.marker_queued, theme.accent)
    } else if flags.in_playlist {
        (icons.dot, theme.orange)
    } else {
        (" ", theme.dim)
    }
}

fn ellipsized(text: &str, width: u16) -> String {
    if text.width() <= width as usize {
        text.to_string()
    } else {
        truncate_end(text, width as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IconMode;
    use crate::ui::icons::icons_for;

    #[test]
    fn layout_reserves_fixed_columns_and_splits_the_rest() {
        let layout = TrackTableLayout::new(100, 8);
        let [marker, index, title, channel, right] = layout.constraints();
        assert_eq!(marker, Constraint::Length(2));
        assert_eq!(index, Constraint::Length(3));
        assert_eq!(right, Constraint::Length(8));
        let (Constraint::Length(title), Constraint::Length(channel)) = (title, channel) else {
            panic!("flexible columns must be lengths");
        };
        assert_eq!(title + channel, 100 - 2 - 3 - 8 - 4 - 1);
        assert!(title > channel, "title gets the larger share");
    }

    #[test]
    fn long_titles_truncate_with_an_ellipsis() {
        let layout = TrackTableLayout::new(40, 8);
        assert!(layout.title_width < 30);
        let text = ellipsized(&"x".repeat(64), layout.title_width);
        assert!(text.ends_with('…'), "got {text:?}");
        assert!(text.chars().count() <= layout.title_width as usize);
    }

    #[test]
    fn marker_priority_is_playing_then_queued_then_playlist() {
        let icons = icons_for(IconMode::Ascii);
        let theme = Theme::from_truecolor(false);
        let all = TrackFlags {
            playing: true,
            queued: true,
            in_playlist: true,
        };
        assert_eq!(marker(all, &theme, &icons).0, icons.play_btn);
        let queued = TrackFlags {
            playing: false,
            queued: true,
            in_playlist: true,
        };
        assert_eq!(marker(queued, &theme, &icons).0, icons.marker_queued);
    }
}
