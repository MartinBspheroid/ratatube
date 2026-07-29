//! Default keybindings (PRD section 9).
//!
//! The keymap will become configurable in v1.1; v1 ships these defaults.

mod help;
mod views;

use crate::app::action::UiMsg;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::{Action, NavigationAction, PlaybackAction};
use crate::app::state::{Focus, PlayingPane, View};
use crate::ui::layout::Breakpoint;

pub use help::HELP_SECTIONS;
pub use views::view_action;

/// Map a key event to a global action, independent of the active view.
pub fn global_action(key: &KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') => Some(Action::Navigation(NavigationAction::Quit)),
            KeyCode::Char('p') => Some(Action::Ui(UiMsg::OpenSettings)),
            _ => None,
        };
    }
    let action = match key.code {
        KeyCode::Char('1') => Action::Ui(UiMsg::Navigate(View::Home)),
        KeyCode::Char('2') => Action::Ui(UiMsg::Navigate(View::Search)),
        KeyCode::Char('3') => Action::Ui(UiMsg::Navigate(View::Queue)),
        KeyCode::Char('4') => Action::Ui(UiMsg::Navigate(View::Playlists)),
        KeyCode::Char('5') => Action::Ui(UiMsg::Navigate(View::History)),
        KeyCode::Char('6') => Action::Ui(UiMsg::Navigate(View::NowPlaying)),
        KeyCode::Tab => Action::Ui(UiMsg::NextView),
        KeyCode::BackTab => Action::Ui(UiMsg::PreviousView),
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
        KeyCode::Char('!') => Action::Ui(UiMsg::ToggleNotificationLog),
        KeyCode::Char('?') => Action::Ui(UiMsg::OpenHelp),
        KeyCode::Char('q') => Action::Navigation(NavigationAction::Quit),
        KeyCode::Char('j') | KeyCode::Down => Action::Ui(UiMsg::SelectNext),
        KeyCode::Char('k') | KeyCode::Up => Action::Ui(UiMsg::SelectPrevious),
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
        KeyCode::Enter | KeyCode::Esc => None,
        KeyCode::Backspace => Some(Action::Navigation(NavigationAction::SearchBackspace)),
        KeyCode::Char(character) => {
            Some(Action::Navigation(NavigationAction::SearchInput(character)))
        }
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
            Some(Action::Ui(UiMsg::CyclePlayingPane))
        }
        (PlayingPane::Queue, KeyCode::Char('j') | KeyCode::Down) => {
            Some(Action::Ui(UiMsg::SelectNext))
        }
        (PlayingPane::Queue, KeyCode::Char('k') | KeyCode::Up) => {
            Some(Action::Ui(UiMsg::SelectPrevious))
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
        Focus::ListFilter => None,
        Focus::Content => view_action(key, view).or_else(|| global_action(key)),
    }
}

#[cfg(test)]
mod tests;
