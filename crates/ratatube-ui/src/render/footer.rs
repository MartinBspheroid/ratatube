//! Context-sensitive keyboard hints.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::render::theme::Theme;
use crate::state::{AppState, View};

/// Render context-sensitive keyboard hints as styled chips.
pub(super) fn render_footer(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let hints: &[(&str, &str)] = match state.ui.view {
        View::Home
            if state.ui.home_section == crate::state::HomeSection::Resume
                && state.domain.pending_resume.is_some() =>
        {
            &[
                ("Space", "resume"),
                ("h/l", "section"),
                ("c", "actions"),
                ("/", "search"),
                ("?", "help"),
            ]
        }
        View::Home if state.ui.home_section == crate::state::HomeSection::Recent => &[
            ("Enter", "play"),
            ("a", "queue"),
            ("P", "playlist"),
            ("h/l", "section"),
            ("c", "actions"),
            ("/", "search"),
            ("?", "help"),
        ],
        View::Home if state.ui.home_section == crate::state::HomeSection::Playlists => &[
            ("h/l", "section"),
            ("Enter", "play all"),
            ("p", "play all"),
            ("N", "new"),
            ("4", "view all"),
            ("?", "help"),
        ],
        View::Home => &[("h/l", "section"), ("/", "search"), ("?", "help")],
        View::Search
            if crate::render::layout::Breakpoint::from_width(state.ui.screen_area.width)
                == crate::render::layout::Breakpoint::Narrow =>
        {
            &[
                ("Enter", "play"),
                ("a", "queue"),
                ("A", "next"),
                ("i", "details"),
                ("c", "actions"),
                ("/", "search"),
                ("?", "help"),
            ]
        }
        View::Search => &[
            ("Enter", "play"),
            ("a", "queue"),
            ("A", "next"),
            ("P", "playlist"),
            ("c", "actions"),
            ("/", "search"),
            ("?", "help"),
        ],
        View::Queue => &[
            ("Enter", "play"),
            ("J/K", "move"),
            ("d", "remove"),
            ("u", "undo"),
            ("w", "save"),
            ("s", "shuffle"),
            ("c", "actions"),
            ("/", "filter"),
            ("C", "clear"),
        ],
        View::Playlists => &[
            ("Enter", "open"),
            ("p", "play"),
            ("N", "new"),
            ("i", "import"),
            ("I", "JSON"),
            ("R", "rename"),
            ("x", "delete"),
            ("/", "filter"),
        ],
        View::PlaylistDetail => &[
            ("Enter", "play"),
            ("p", "play all"),
            ("J/K", "move"),
            ("d", "remove"),
            ("e", "edit details"),
            ("P", "copy to"),
            ("c", "actions"),
            ("Bksp", "back"),
        ],
        View::Channel => &[
            ("Enter", "play/load"),
            ("a/A", "queue/next"),
            ("P", "playlist"),
            ("c", "actions"),
            ("Bksp", "back"),
        ],
        View::History => match state.ui.history_view_mode {
            crate::state::HistoryViewMode::Recent => &[
                ("Enter", "replay"),
                ("a", "queue"),
                ("P", "playlist"),
                ("x", "delete"),
                ("g", "top"),
                ("c", "actions"),
                ("/", "filter"),
                ("C", "clear"),
            ],
            crate::state::HistoryViewMode::Top => &[
                ("Enter", "replay"),
                ("a", "queue"),
                ("P", "playlist"),
                ("g", "recent"),
                ("c", "actions"),
                ("/", "filter"),
                ("C", "clear"),
            ],
        },
        View::NowPlaying
            if crate::render::layout::Breakpoint::from_width(state.ui.screen_area.width)
                == crate::render::layout::Breakpoint::UltraWide
                && state.ui.playing_pane == crate::state::PlayingPane::Queue =>
        {
            &[
                ("h/l", "info/queue"),
                ("j/k", "select"),
                ("Enter", "play"),
                ("Space", "pause"),
                ("c", "actions"),
            ]
        }
        View::NowPlaying
            if crate::render::layout::Breakpoint::from_width(state.ui.screen_area.width)
                == crate::render::layout::Breakpoint::UltraWide =>
        {
            &[
                ("h/l", "info/queue"),
                ("j/k", "scroll"),
                ("Space", "pause"),
                ("./,", "chapter"),
                ("c", "actions"),
                ("v", "pane"),
            ]
        }
        View::NowPlaying => &[
            ("Space", "pause"),
            ("h/l", "seek"),
            ("j/k", "scroll"),
            ("./,", "chapter"),
            ("n/b", "next/prev"),
            ("c", "actions"),
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
