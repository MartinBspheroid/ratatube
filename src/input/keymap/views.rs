//! View-specific key bindings.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::action::{
    Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction,
};
use crate::app::state::View;

/// Map view-owned bindings before global bindings.
pub fn view_action(key: &KeyEvent, view: View) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    let action = match (view, key.code) {
        (
            View::Home
            | View::Search
            | View::Queue
            | View::PlaylistDetail
            | View::Channel
            | View::History
            | View::NowPlaying,
            KeyCode::Char('c'),
        ) => Action::Navigation(NavigationAction::OpenTrackContext),
        (View::Channel, KeyCode::Backspace | KeyCode::Esc) => {
            Action::Navigation(NavigationAction::BackFromChannel)
        }
        (View::Channel, KeyCode::Char('a')) => Action::Queue(QueueAction::AddSelectedToQueue),
        (View::Channel, KeyCode::Char('A')) => Action::Queue(QueueAction::AddSelectedAsNext),
        (View::Channel, KeyCode::Char('P')) => {
            Action::Playlists(PlaylistAction::OpenPlaylistPicker)
        }
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
        (View::Queue, KeyCode::Char('C')) => Action::Queue(QueueAction::ClearQueue),
        (View::Queue, KeyCode::Char('u')) => Action::Queue(QueueAction::UndoQueueRemoval),
        (View::Queue, KeyCode::Char('J')) => Action::Queue(QueueAction::MoveSelectedInQueue(1)),
        (View::Queue, KeyCode::Char('K')) => Action::Queue(QueueAction::MoveSelectedInQueue(-1)),
        (View::Queue, KeyCode::Char('P')) => Action::Playlists(PlaylistAction::OpenPlaylistPicker),
        (View::Queue, KeyCode::Char('w')) => Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::SaveQueueAsPlaylist,
        )),
        (View::Playlists, KeyCode::Enter | KeyCode::Char('v')) => {
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
        (View::NowPlaying, KeyCode::Char('j') | KeyCode::Down) => {
            Action::Playback(PlaybackAction::ScrollNowPlaying(3))
        }
        (View::NowPlaying, KeyCode::Char('k') | KeyCode::Up) => {
            Action::Playback(PlaybackAction::ScrollNowPlaying(-3))
        }
        (View::NowPlaying, KeyCode::Char('d') | KeyCode::PageDown) => {
            Action::Playback(PlaybackAction::ScrollNowPlaying(15))
        }
        (View::NowPlaying, KeyCode::Char('u') | KeyCode::PageUp) => {
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
        (View::Search | View::NowPlaying, KeyCode::Char('o')) => {
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
        (View::History, KeyCode::Char('C')) => Action::History(HistoryAction::ClearHistory),
        (View::History, KeyCode::Char('g')) => {
            Action::History(HistoryAction::ToggleHistoryViewMode)
        }
        _ => return None,
    };
    Some(action)
}
