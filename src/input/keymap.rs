//! Default keybindings (PRD section 9).
//!
//! The keymap will become configurable in v1.1; v1 ships these defaults.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::{
    Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction,
};
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
            return Some(Action::Navigation(NavigationAction::Quit));
        }
        return None;
    }
    let action = match key.code {
        KeyCode::Char('1') => Action::Navigation(NavigationAction::Navigate(View::Home)),
        KeyCode::Char('2') => Action::Navigation(NavigationAction::Navigate(View::Search)),
        KeyCode::Char('3') => Action::Navigation(NavigationAction::Navigate(View::Queue)),
        KeyCode::Char('4') => Action::Navigation(NavigationAction::Navigate(View::Playlists)),
        KeyCode::Char('5') => Action::Navigation(NavigationAction::Navigate(View::History)),
        KeyCode::Char('6') => Action::Navigation(NavigationAction::Navigate(View::NowPlaying)),
        KeyCode::Tab => Action::Navigation(NavigationAction::NextView),
        KeyCode::BackTab => Action::Navigation(NavigationAction::PreviousView),
        KeyCode::Char(' ') => Action::Playback(PlaybackAction::PlayPause),
        KeyCode::Char('n') => Action::Playback(PlaybackAction::NextTrack),
        KeyCode::Char('b') => Action::Playback(PlaybackAction::PreviousTrack),
        KeyCode::Char('+') => Action::Playback(PlaybackAction::VolumeUp),
        KeyCode::Char('-') => Action::Playback(PlaybackAction::VolumeDown),
        KeyCode::Char('m') => Action::Playback(PlaybackAction::ToggleMute),
        KeyCode::Char('s') => Action::Playback(PlaybackAction::ToggleShuffle),
        KeyCode::Char('r') => Action::Playback(PlaybackAction::CycleRepeat),
        KeyCode::Char('h') => Action::Playback(PlaybackAction::SeekBackward),
        KeyCode::Char('l') => Action::Playback(PlaybackAction::SeekForward),
        KeyCode::Char('H') => Action::Playback(PlaybackAction::SeekBackwardLarge),
        KeyCode::Char('L') => Action::Playback(PlaybackAction::SeekForwardLarge),
        KeyCode::Char('.') => Action::Playback(PlaybackAction::NextChapter),
        KeyCode::Char(',') => Action::Playback(PlaybackAction::PreviousChapter),
        KeyCode::Char('>') => Action::Playback(PlaybackAction::SpeedUp),
        KeyCode::Char('<') => Action::Playback(PlaybackAction::SpeedDown),
        KeyCode::Char('=') => Action::Playback(PlaybackAction::SpeedReset),
        KeyCode::Char('Z') => Action::Playback(PlaybackAction::CycleSleepTimer),
        KeyCode::Char('t') => Action::Playback(PlaybackAction::ToggleRadio),
        KeyCode::Char('!') => Action::History(HistoryAction::ToggleNotificationLog),
        KeyCode::Char('?') => Action::Navigation(NavigationAction::OpenHelp),
        KeyCode::Char('q') => Action::Navigation(NavigationAction::Quit),
        KeyCode::Char('j') | KeyCode::Down => Action::Navigation(NavigationAction::SelectNext),
        KeyCode::Char('k') | KeyCode::Up => Action::Navigation(NavigationAction::SelectPrevious),
        KeyCode::Enter => Action::Playback(PlaybackAction::PlaySelected),
        _ => return None,
    };
    Some(action)
}

/// Map a key event while the search input has focus (PRD 9: `/` focuses it).
pub fn search_input_action(key: &KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') = key.code {
            return Some(Action::Navigation(NavigationAction::Quit));
        }
        return None;
    }
    match key.code {
        KeyCode::Enter | KeyCode::Esc => None, // handled by the app layer
        KeyCode::Backspace => Some(Action::Navigation(NavigationAction::SearchBackspace)),
        KeyCode::Char(c) => Some(Action::Navigation(NavigationAction::SearchInput(c))),
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
            Action::Navigation(NavigationAction::CycleHomeSection(-1))
        }
        (View::Home, KeyCode::Char('l')) | (View::Home, KeyCode::Right) => {
            Action::Navigation(NavigationAction::CycleHomeSection(1))
        }
        (View::Home, KeyCode::Char('a')) => Action::Queue(QueueAction::AddSelectedToQueue),
        (View::Home, KeyCode::Char('p')) => {
            Action::Queue(QueueAction::LoadSelectedPlaylistIntoQueue)
        }
        (View::Home, KeyCode::Char('P')) => Action::Playlists(PlaylistAction::OpenPlaylistPicker),
        (View::Home, KeyCode::Char('N')) => Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::NewPlaylist,
        )),
        (View::Queue, KeyCode::Char('d')) => Action::Queue(QueueAction::RemoveSelectedFromQueue),
        (View::Queue, KeyCode::Char('u')) => Action::Queue(QueueAction::UndoQueueRemoval),
        (View::Queue, KeyCode::Char('J')) => Action::Queue(QueueAction::MoveSelectedInQueue(1)),
        (View::Queue, KeyCode::Char('K')) => Action::Queue(QueueAction::MoveSelectedInQueue(-1)),
        (View::Queue, KeyCode::Char('P')) => Action::Playlists(PlaylistAction::OpenPlaylistPicker),
        (View::Queue, KeyCode::Char('c')) => Action::Queue(QueueAction::ClearQueue),
        (View::Queue, KeyCode::Char('w')) => Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::SaveQueueAsPlaylist,
        )),
        (View::Playlists, KeyCode::Enter) => Action::Playlists(PlaylistAction::OpenPlaylistDetail),
        (View::Playlists, KeyCode::Char('v')) => {
            Action::Playlists(PlaylistAction::OpenPlaylistDetail)
        }
        (View::Playlists, KeyCode::Char('p')) => {
            Action::Queue(QueueAction::LoadSelectedPlaylistIntoQueue)
        }
        (View::Playlists, KeyCode::Char('a')) => {
            Action::Queue(QueueAction::AppendSelectedPlaylistToQueue)
        }
        (View::Playlists, KeyCode::Char('x')) => {
            Action::Playlists(PlaylistAction::DeleteSelectedPlaylist)
        }
        (View::Playlists, KeyCode::Char('i')) => Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::ImportPlaylistUrl,
        )),
        (View::Playlists, KeyCode::Char('I')) => Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::ImportPlaylistJson,
        )),
        (View::Playlists, KeyCode::Char('N')) => Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::NewPlaylist,
        )),
        (View::Playlists, KeyCode::Char('R')) => Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::RenamePlaylist,
        )),
        (View::PlaylistDetail, KeyCode::Char('p')) => {
            Action::Queue(QueueAction::LoadSelectedPlaylistIntoQueue)
        }
        (View::PlaylistDetail, KeyCode::Char('a')) => {
            Action::Queue(QueueAction::AddSelectedToQueue)
        }
        (View::PlaylistDetail, KeyCode::Char('A')) => Action::Queue(QueueAction::AddSelectedAsNext),
        (View::PlaylistDetail, KeyCode::Char('d')) => {
            Action::Playlists(PlaylistAction::RemoveSelectedFromPlaylist)
        }
        (View::PlaylistDetail, KeyCode::Char('J')) => {
            Action::Playlists(PlaylistAction::MoveSelectedInPlaylist(1))
        }
        (View::PlaylistDetail, KeyCode::Char('K')) => {
            Action::Playlists(PlaylistAction::MoveSelectedInPlaylist(-1))
        }
        (View::PlaylistDetail, KeyCode::Char('P')) => {
            Action::Playlists(PlaylistAction::OpenPlaylistPicker)
        }
        (View::PlaylistDetail, KeyCode::Char('e')) => {
            Action::Playlists(PlaylistAction::OpenPlaylistEditor)
        }
        (View::PlaylistDetail, KeyCode::Backspace) => {
            Action::Navigation(NavigationAction::Navigate(View::Playlists))
        }
        (View::NowPlaying, KeyCode::Char('j')) | (View::NowPlaying, KeyCode::Down) => {
            Action::Playback(PlaybackAction::ScrollNowPlaying(3))
        }
        (View::NowPlaying, KeyCode::Char('k')) | (View::NowPlaying, KeyCode::Up) => {
            Action::Playback(PlaybackAction::ScrollNowPlaying(-3))
        }
        (View::NowPlaying, KeyCode::Char('d')) | (View::NowPlaying, KeyCode::PageDown) => {
            Action::Playback(PlaybackAction::ScrollNowPlaying(15))
        }
        (View::NowPlaying, KeyCode::Char('u')) | (View::NowPlaying, KeyCode::PageUp) => {
            Action::Playback(PlaybackAction::ScrollNowPlaying(-15))
        }
        (View::NowPlaying, KeyCode::Char('v')) => {
            Action::Playback(PlaybackAction::ToggleNowPlayingPane)
        }
        (View::Help, KeyCode::Esc | KeyCode::Char('?')) => {
            Action::Navigation(NavigationAction::CloseHelp)
        }
        (View::Help, KeyCode::Char('j') | KeyCode::Down) => {
            Action::Navigation(NavigationAction::ScrollHelp(1))
        }
        (View::Help, KeyCode::Char('k') | KeyCode::Up) => {
            Action::Navigation(NavigationAction::ScrollHelp(-1))
        }
        (View::Help, KeyCode::PageDown) => Action::Navigation(NavigationAction::ScrollHelp(10)),
        (View::Help, KeyCode::PageUp) => Action::Navigation(NavigationAction::ScrollHelp(-10)),
        (View::Search, KeyCode::Char('a')) => Action::Queue(QueueAction::AddSelectedToQueue),
        (View::Search, KeyCode::Char('A')) => Action::Queue(QueueAction::AddSelectedAsNext),
        (View::Search, KeyCode::Char('P')) => Action::Playlists(PlaylistAction::OpenPlaylistPicker),
        (View::Search, KeyCode::Char('i')) => {
            Action::Navigation(NavigationAction::ToggleSearchDetail)
        }
        (View::Search, KeyCode::Char('o')) => Action::Navigation(NavigationAction::OpenInBrowser),
        (View::NowPlaying, KeyCode::Char('o')) => {
            Action::Navigation(NavigationAction::OpenInBrowser)
        }
        (View::History, KeyCode::Char('a')) => Action::Queue(QueueAction::AddSelectedToQueue),
        (View::History, KeyCode::Char('A')) => Action::Queue(QueueAction::AddSelectedAsNext),
        (View::History, KeyCode::Char('P')) => {
            Action::Playlists(PlaylistAction::OpenPlaylistPicker)
        }
        (View::History, KeyCode::Char('x')) => {
            Action::History(HistoryAction::DeleteSelectedHistoryEntry)
        }
        (View::History, KeyCode::Char('g')) => {
            Action::History(HistoryAction::ToggleHistoryViewMode)
        }
        (View::History, KeyCode::Char('c')) => Action::History(HistoryAction::ClearHistory),
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
            Some(Action::Playback(PlaybackAction::CyclePlayingPane))
        }
        (PlayingPane::Queue, KeyCode::Char('j') | KeyCode::Down) => {
            Some(Action::Navigation(NavigationAction::SelectNext))
        }
        (PlayingPane::Queue, KeyCode::Char('k') | KeyCode::Up) => {
            Some(Action::Navigation(NavigationAction::SelectPrevious))
        }
        (PlayingPane::Queue, KeyCode::Enter) => {
            Some(Action::Playback(PlaybackAction::PlaySelected))
        }
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
            Some(Action::Playlists(PlaylistAction::OpenPrompt(
                crate::app::state::PromptPurpose::NewPlaylist
            )))
        ));
    }

    #[test]
    fn playlist_action_row_matches_real_bindings() {
        let action = |code| view_action(&KeyEvent::new(code, KeyModifiers::NONE), View::Playlists);
        assert!(matches!(
            action(KeyCode::Enter),
            Some(Action::Playlists(PlaylistAction::OpenPlaylistDetail))
        ));
        assert!(matches!(
            action(KeyCode::Char('N')),
            Some(Action::Playlists(PlaylistAction::OpenPrompt(
                crate::app::state::PromptPurpose::NewPlaylist
            )))
        ));
        assert!(matches!(
            action(KeyCode::Char('i')),
            Some(Action::Playlists(PlaylistAction::OpenPrompt(
                crate::app::state::PromptPurpose::ImportPlaylistUrl
            )))
        ));
        assert!(matches!(
            action(KeyCode::Char('I')),
            Some(Action::Playlists(PlaylistAction::OpenPrompt(
                crate::app::state::PromptPurpose::ImportPlaylistJson
            )))
        ));
        assert!(matches!(
            action(KeyCode::Char('R')),
            Some(Action::Playlists(PlaylistAction::OpenPrompt(
                crate::app::state::PromptPurpose::RenamePlaylist
            )))
        ));
        assert!(matches!(
            action(KeyCode::Char('x')),
            Some(Action::Playlists(PlaylistAction::DeleteSelectedPlaylist))
        ));
        assert!(matches!(
            view_action(
                &KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
                View::PlaylistDetail
            ),
            Some(Action::Playlists(PlaylistAction::OpenPlaylistEditor))
        ));
    }

    #[test]
    fn help_close_and_scroll_bindings_take_precedence_over_globals() {
        let close = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(matches!(
            route(&close, Focus::Content, View::Help),
            Some(Action::Navigation(NavigationAction::CloseHelp))
        ));
        assert!(matches!(
            route(&down, Focus::Content, View::Help),
            Some(Action::Navigation(NavigationAction::ScrollHelp(1)))
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
            Some(Action::Playback(PlaybackAction::CyclePlayingPane))
        ));
        assert!(matches!(
            playing_pane_action(
                &key(KeyCode::Down),
                PlayingPane::Queue,
                Breakpoint::UltraWide
            ),
            Some(Action::Navigation(NavigationAction::SelectNext))
        ));
        assert!(matches!(
            playing_pane_action(
                &key(KeyCode::Enter),
                PlayingPane::Queue,
                Breakpoint::UltraWide
            ),
            Some(Action::Playback(PlaybackAction::PlaySelected))
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
            Some(Action::Navigation(NavigationAction::ToggleSearchDetail))
        ));
        assert!(matches!(
            route(&key('o'), Focus::Content, View::Search),
            Some(Action::Navigation(NavigationAction::OpenInBrowser))
        ));
        assert!(matches!(
            route(&key('o'), Focus::Content, View::NowPlaying),
            Some(Action::Navigation(NavigationAction::OpenInBrowser))
        ));
    }
}
