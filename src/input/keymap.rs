//! Default keybindings (PRD section 9).
//!
//! The keymap will become configurable in v1.1; v1 ships these defaults.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::Action;
use crate::app::state::{Focus, View};

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
        KeyCode::Char('?') => Action::Navigate(View::Help),
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
        (View::Queue, KeyCode::Char('d')) => Action::RemoveSelectedFromQueue,
        (View::Queue, KeyCode::Char('J')) => Action::MoveSelectedInQueue(1),
        (View::Queue, KeyCode::Char('K')) => Action::MoveSelectedInQueue(-1),
        (View::Queue, KeyCode::Char('c')) => Action::ClearQueue,
        (View::Queue, KeyCode::Char('w')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::SaveQueueAsPlaylist)
        }
        (View::Playlists, KeyCode::Enter) => Action::OpenPlaylistDetail,
        (View::Playlists, KeyCode::Char('p')) => Action::LoadSelectedPlaylistIntoQueue,
        (View::Playlists, KeyCode::Char('a')) => Action::AppendSelectedPlaylistToQueue,
        (View::Playlists, KeyCode::Char('x')) => Action::DeleteSelectedPlaylist,
        (View::Playlists, KeyCode::Char('i')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::ImportPlaylistUrl)
        }
        (View::Playlists, KeyCode::Char('N')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::NewPlaylist)
        }
        (View::Playlists, KeyCode::Char('R')) => {
            Action::OpenPrompt(crate::app::state::PromptPurpose::RenamePlaylist)
        }
        (View::PlaylistDetail, KeyCode::Char('p')) => Action::LoadSelectedPlaylistIntoQueue,
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
        (View::Search, KeyCode::Char('a')) => Action::AddSelectedToQueue,
        (View::Search, KeyCode::Char('A')) => Action::AddSelectedAsNext,
        (View::History, KeyCode::Char('a')) => Action::AddSelectedToQueue,
        (View::History, KeyCode::Char('c')) => Action::ClearHistory,
        _ => return None,
    };
    Some(action)
}

/// Route a key event based on current focus.
pub fn route(key: &KeyEvent, focus: Focus, view: View) -> Option<Action> {
    match focus {
        Focus::SearchInput => search_input_action(key),
        Focus::Content => view_action(key, view).or_else(|| global_action(key)),
    }
}
