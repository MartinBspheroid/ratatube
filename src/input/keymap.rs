//! Default keybindings (PRD section 9).
//!
//! The keymap will become configurable in v1.1; v1 ships these defaults.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::Action;
use crate::app::state::{Focus, PlayingPane, View};
use crate::ui::layout::Breakpoint;

/// Canonical user-facing command catalog rendered by the Help view.
pub const HELP_SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Views",
        &[
            ("1", "Home"),
            ("2", "Search"),
            ("3", "Queue"),
            ("4", "Playlists"),
            ("5", "History"),
            ("6", "Now Playing"),
        ],
    ),
    (
        "Playback",
        &[
            ("Space", "Play / pause (resume on Home)"),
            ("n / b", "Next / previous track"),
            (". / ,", "Next / previous chapter"),
            ("h / l", "Seek 5 seconds"),
            ("H / L", "Seek 30 seconds"),
            ("+ / -", "Volume"),
            ("m", "Mute"),
            ("s", "Shuffle"),
            ("r", "Repeat mode"),
            ("t", "Radio mode (auto-refill queue)"),
            ("< / >", "Playback speed (= resets)"),
            ("Z", "Sleep timer 15/30/60 min"),
        ],
    ),
    (
        "Lists",
        &[
            ("j / k", "Move selection"),
            ("Enter", "Play / open"),
            ("a / A", "Add to queue / play next"),
            ("J / K", "Move item down / up"),
            ("d / u", "Remove queue item / undo removal"),
            ("P", "Add to playlist..."),
            ("/", "Filter the list"),
        ],
    ),
    (
        "Playlists",
        &[
            ("Enter", "Open playlist editor"),
            ("p", "Play playlist"),
            ("i", "Import from URL"),
            ("I", "Import pasted JSON"),
            ("e", "Edit playlist name and description"),
            ("N", "New playlist"),
            ("R", "Rename"),
            ("x", "Delete (asks first)"),
            ("w", "Save queue as playlist"),
        ],
    ),
    (
        "History",
        &[("g", "Toggle recent / top"), ("x", "Delete entry")],
    ),
    (
        "Other",
        &[
            ("/", "Search (outside lists)"),
            ("!", "Message log"),
            ("v", "Chapters / description pane"),
            ("o", "Open selected/current track in browser"),
            ("?", "This help / return"),
            ("q", "Quit"),
        ],
    ),
];

/// Map a key event to a global action, independent of the active view.
pub fn global_action(key: &KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') = key.code {
            return Some(Action::Quit);
        }
        return None;
    }
    let action = match key.code {
        KeyCode::Char('1') => Action::Navigate(View::Home),
        KeyCode::Char('2') => Action::Navigate(View::Search),
        KeyCode::Char('3') => Action::Navigate(View::Queue),
        KeyCode::Char('4') => Action::Navigate(View::Playlists),
        KeyCode::Char('5') => Action::Navigate(View::History),
        KeyCode::Char('6') => Action::Navigate(View::NowPlaying),
        KeyCode::Tab => Action::NextView,
        KeyCode::BackTab => Action::PreviousView,
        KeyCode::Char(' ') => Action::PlayPause,
        KeyCode::Char('n') => Action::NextTrack,
        KeyCode::Char('b') => Action::PreviousTrack,
        KeyCode::Char('+') => Action::VolumeUp,
        KeyCode::Char('-') => Action::VolumeDown,
        KeyCode::Char('m') => Action::ToggleMute,
        KeyCode::Char('s') => Action::ToggleShuffle,
        KeyCode::Char('r') => Action::CycleRepeat,
        KeyCode::Char('h') => Action::SeekBackward,
        KeyCode::Char('l') => Action::SeekForward,
        KeyCode::Char('H') => Action::SeekBackwardLarge,
        KeyCode::Char('L') => Action::SeekForwardLarge,
        KeyCode::Char('.') => Action::NextChapter,
        KeyCode::Char(',') => Action::PreviousChapter,
        KeyCode::Char('>') => Action::SpeedUp,
        KeyCode::Char('<') => Action::SpeedDown,
        KeyCode::Char('=') => Action::SpeedReset,
        KeyCode::Char('Z') => Action::CycleSleepTimer,
        KeyCode::Char('t') => Action::ToggleRadio,
        KeyCode::Char('!') => Action::ToggleNotificationLog,
        KeyCode::Char('?') => Action::OpenHelp,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::SelectNext,
        KeyCode::Char('k') | KeyCode::Up => Action::SelectPrevious,
        KeyCode::Enter => Action::PlaySelected,
        _ => return None,
    };
    Some(action)
}

/// Map a key event while the search input has focus (PRD 9: `/` focuses it).
pub fn search_input_action(key: &KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') = key.code {
            return Some(Action::Quit);
        }
        return None;
    }
    match key.code {
        KeyCode::Enter | KeyCode::Esc => None, // handled by the app layer
        KeyCode::Backspace => Some(Action::SearchBackspace),
        KeyCode::Char(c) => Some(Action::SearchInput(c)),
        _ => None,
    }
}

/// Whether `key` requests focusing the search input.
pub fn focuses_search(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('/') && key.modifiers.is_empty()
}

/// Whether `key` leaves search-input focus without submitting.
pub fn leaves_search(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc)
}

/// Whether `key` submits the search input.
pub fn submits_search(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Enter)
}

/// View-specific bindings, checked before global ones.
pub fn view_action(key: &KeyEvent, view: View) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    let action = match (view, key.code) {
        // Home: h/l move between sections instead of seeking; seeking from
        // the dashboard is a non-goal.
        (View::Home, KeyCode::Char('h')) | (View::Home, KeyCode::Left) => {
            Action::CycleHomeSection(-1)
        }
        (View::Home, KeyCode::Char('l')) | (View::Home, KeyCode::Right) => {
            Action::CycleHomeSection(1)
        }
        (View::Home, KeyCode::Char('a')) => Action::AddSelectedToQueue,
        (View::Home, KeyCode::Char('p')) => Action::LoadSelectedPlaylistIntoQueue,
        (View::Home, KeyCode::Char('P')) => Action::OpenPlaylistPicker,
        (View::Home, KeyCode::Char('N')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::NewPlaylist)
        }
        (View::Queue, KeyCode::Char('d')) => Action::RemoveSelectedFromQueue,
        (View::Queue, KeyCode::Char('u')) => Action::UndoQueueRemoval,
        (View::Queue, KeyCode::Char('J')) => Action::MoveSelectedInQueue(1),
        (View::Queue, KeyCode::Char('K')) => Action::MoveSelectedInQueue(-1),
        (View::Queue, KeyCode::Char('P')) => Action::OpenPlaylistPicker,
        (View::Queue, KeyCode::Char('c')) => Action::ClearQueue,
        (View::Queue, KeyCode::Char('w')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::SaveQueueAsPlaylist)
        }
        (View::Playlists, KeyCode::Enter) => Action::OpenPlaylistDetail,
        (View::Playlists, KeyCode::Char('v')) => Action::OpenPlaylistDetail,
        (View::Playlists, KeyCode::Char('p')) => Action::LoadSelectedPlaylistIntoQueue,
        (View::Playlists, KeyCode::Char('a')) => Action::AppendSelectedPlaylistToQueue,
        (View::Playlists, KeyCode::Char('x')) => Action::DeleteSelectedPlaylist,
        (View::Playlists, KeyCode::Char('i')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::ImportPlaylistUrl)
        }
        (View::Playlists, KeyCode::Char('I')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::ImportPlaylistJson)
        }
        (View::Playlists, KeyCode::Char('N')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::NewPlaylist)
        }
        (View::Playlists, KeyCode::Char('R')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::RenamePlaylist)
        }
        (View::PlaylistDetail, KeyCode::Char('p')) => Action::LoadSelectedPlaylistIntoQueue,
        (View::PlaylistDetail, KeyCode::Char('a')) => Action::AddSelectedToQueue,
        (View::PlaylistDetail, KeyCode::Char('A')) => Action::AddSelectedAsNext,
        (View::PlaylistDetail, KeyCode::Char('d')) => Action::RemoveSelectedFromPlaylist,
        (View::PlaylistDetail, KeyCode::Char('J')) => Action::MoveSelectedInPlaylist(1),
        (View::PlaylistDetail, KeyCode::Char('K')) => Action::MoveSelectedInPlaylist(-1),
        (View::PlaylistDetail, KeyCode::Char('P')) => Action::OpenPlaylistPicker,
        (View::PlaylistDetail, KeyCode::Char('e')) => Action::OpenPlaylistEditor,
        (View::PlaylistDetail, KeyCode::Backspace) => Action::Navigate(View::Playlists),
        (View::NowPlaying, KeyCode::Char('j')) | (View::NowPlaying, KeyCode::Down) => {
            Action::ScrollNowPlaying(3)
        }
        (View::NowPlaying, KeyCode::Char('k')) | (View::NowPlaying, KeyCode::Up) => {
            Action::ScrollNowPlaying(-3)
        }
        (View::NowPlaying, KeyCode::Char('d')) | (View::NowPlaying, KeyCode::PageDown) => {
            Action::ScrollNowPlaying(15)
        }
        (View::NowPlaying, KeyCode::Char('u')) | (View::NowPlaying, KeyCode::PageUp) => {
            Action::ScrollNowPlaying(-15)
        }
        (View::NowPlaying, KeyCode::Char('v')) => Action::ToggleNowPlayingPane,
        (View::Help, KeyCode::Esc | KeyCode::Char('?')) => Action::CloseHelp,
        (View::Help, KeyCode::Char('j') | KeyCode::Down) => Action::ScrollHelp(1),
        (View::Help, KeyCode::Char('k') | KeyCode::Up) => Action::ScrollHelp(-1),
        (View::Help, KeyCode::PageDown) => Action::ScrollHelp(10),
        (View::Help, KeyCode::PageUp) => Action::ScrollHelp(-10),
        (View::Search, KeyCode::Char('a')) => Action::AddSelectedToQueue,
        (View::Search, KeyCode::Char('A')) => Action::AddSelectedAsNext,
        (View::Search, KeyCode::Char('P')) => Action::OpenPlaylistPicker,
        (View::Search, KeyCode::Char('i')) => Action::ToggleSearchDetail,
        (View::Search, KeyCode::Char('o')) => Action::OpenInBrowser,
        (View::NowPlaying, KeyCode::Char('o')) => Action::OpenInBrowser,
        (View::History, KeyCode::Char('a')) => Action::AddSelectedToQueue,
        (View::History, KeyCode::Char('A')) => Action::AddSelectedAsNext,
        (View::History, KeyCode::Char('P')) => Action::OpenPlaylistPicker,
        (View::History, KeyCode::Char('x')) => Action::DeleteSelectedHistoryEntry,
        (View::History, KeyCode::Char('g')) => Action::ToggleHistoryViewMode,
        (View::History, KeyCode::Char('c')) => Action::ClearHistory,
        _ => return None,
    };
    Some(action)
}

/// Route keys owned by the queue pane in the ultra-wide Playing layout.
pub fn playing_pane_action(
    key: &KeyEvent,
    pane: PlayingPane,
    breakpoint: Breakpoint,
) -> Option<Action> {
    if breakpoint != Breakpoint::UltraWide || key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match (pane, key.code) {
        (_, KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Left | KeyCode::Right) => {
            Some(Action::CyclePlayingPane)
        }
        (PlayingPane::Queue, KeyCode::Char('j') | KeyCode::Down) => Some(Action::SelectNext),
        (PlayingPane::Queue, KeyCode::Char('k') | KeyCode::Up) => Some(Action::SelectPrevious),
        (PlayingPane::Queue, KeyCode::Enter) => Some(Action::PlaySelected),
        _ => None,
    }
}

/// Route a key event based on current focus.
pub fn route(key: &KeyEvent, focus: Focus, view: View) -> Option<Action> {
    match focus {
        Focus::SearchInput => search_input_action(key),
        // The in-list filter bar is handled directly by the app layer.
        Focus::ListFilter => None,
        Focus::Content => view_action(key, view).or_else(|| global_action(key)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_new_playlist_hint_has_a_working_binding() {
        let key = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::NONE);
        assert!(matches!(
            route(&key, Focus::Content, View::Home),
            Some(Action::OpenPrompt(
                crate::app::state::PromptPurpose::NewPlaylist
            ))
        ));
    }

    #[test]
    fn playlist_action_row_matches_real_bindings() {
        let action = |code| view_action(&KeyEvent::new(code, KeyModifiers::NONE), View::Playlists);
        assert!(matches!(
            action(KeyCode::Enter),
            Some(Action::OpenPlaylistDetail)
        ));
        assert!(matches!(
            action(KeyCode::Char('N')),
            Some(Action::OpenPrompt(
                crate::app::state::PromptPurpose::NewPlaylist
            ))
        ));
        assert!(matches!(
            action(KeyCode::Char('i')),
            Some(Action::OpenPrompt(
                crate::app::state::PromptPurpose::ImportPlaylistUrl
            ))
        ));
        assert!(matches!(
            action(KeyCode::Char('I')),
            Some(Action::OpenPrompt(
                crate::app::state::PromptPurpose::ImportPlaylistJson
            ))
        ));
        assert!(matches!(
            action(KeyCode::Char('R')),
            Some(Action::OpenPrompt(
                crate::app::state::PromptPurpose::RenamePlaylist
            ))
        ));
        assert!(matches!(
            action(KeyCode::Char('x')),
            Some(Action::DeleteSelectedPlaylist)
        ));
        assert!(matches!(
            view_action(
                &KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
                View::PlaylistDetail
            ),
            Some(Action::OpenPlaylistEditor)
        ));
    }

    #[test]
    fn help_close_and_scroll_bindings_take_precedence_over_globals() {
        let close = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(matches!(
            route(&close, Focus::Content, View::Help),
            Some(Action::CloseHelp)
        ));
        assert!(matches!(
            route(&down, Focus::Content, View::Help),
            Some(Action::ScrollHelp(1))
        ));
    }

    #[test]
    fn ultra_wide_queue_pane_owns_navigation_and_play() {
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        assert!(matches!(
            playing_pane_action(
                &key(KeyCode::Char('l')),
                PlayingPane::Info,
                Breakpoint::UltraWide
            ),
            Some(Action::CyclePlayingPane)
        ));
        assert!(matches!(
            playing_pane_action(
                &key(KeyCode::Down),
                PlayingPane::Queue,
                Breakpoint::UltraWide
            ),
            Some(Action::SelectNext)
        ));
        assert!(matches!(
            playing_pane_action(
                &key(KeyCode::Enter),
                PlayingPane::Queue,
                Breakpoint::UltraWide
            ),
            Some(Action::PlaySelected)
        ));
        assert!(
            playing_pane_action(
                &key(KeyCode::Char('l')),
                PlayingPane::Info,
                Breakpoint::Wide
            )
            .is_none()
        );
    }

    #[test]
    fn search_detail_and_browser_keys_dispatch_real_actions() {
        let key = |character| KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
        assert!(matches!(
            route(&key('i'), Focus::Content, View::Search),
            Some(Action::ToggleSearchDetail)
        ));
        assert!(matches!(
            route(&key('o'), Focus::Content, View::Search),
            Some(Action::OpenInBrowser)
        ));
        assert!(matches!(
            route(&key('o'), Focus::Content, View::NowPlaying),
            Some(Action::OpenInBrowser)
        ));
    }
}
