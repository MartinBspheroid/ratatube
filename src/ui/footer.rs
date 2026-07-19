//! Context-sensitive keyboard hints.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::state::{AppState, View};
use crate::ui::theme::Theme;

/// Render context-sensitive keyboard hints as styled chips.
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
            ("C", "clear"),
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
        View::Channel => &[
            ("Enter", "play/load"),
            ("c", "actions"),
            ("a/A", "queue/next"),
            ("P", "playlist"),
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
                ("C", "clear"),
            ],
            crate::app::state::HistoryViewMode::Top => &[
                ("Enter", "replay"),
                ("a", "queue"),
                ("P", "playlist"),
                ("/", "filter"),
                ("g", "recent"),
                ("C", "clear"),
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
    let spans = hints
        .iter()
        .flat_map(|(key, label)| {
            [
                Span::styled(format!(" {key} "), theme.key_chip),
                Span::styled(format!("{label}  "), theme.dim),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
