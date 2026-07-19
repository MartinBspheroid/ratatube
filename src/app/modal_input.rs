//! Modal-first keyboard routing.

use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction, PlaylistAction};
use crate::app::state::ImportState;

impl App {
    /// Route one modal-owned key and report whether an overlay consumed input.
    pub(super) async fn handle_modal_key(
        &mut self,
        key: &KeyEvent,
        action_tx: &mpsc::Sender<Action>,
    ) -> bool {
        if self.state.track_context_menu.is_some() {
            let action = match key.code {
                KeyCode::Esc => Some(Action::Navigation(NavigationAction::CloseTrackContext)),
                KeyCode::Char('j') | KeyCode::Down => {
                    Some(Action::Navigation(NavigationAction::MoveTrackContext(1)))
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    Some(Action::Navigation(NavigationAction::MoveTrackContext(-1)))
                }
                KeyCode::Enter => Some(Action::Navigation(NavigationAction::SubmitTrackContext)),
                _ => None,
            };
            send_if_some(action_tx, action).await;
            return true;
        }
        if self.state.track_details_modal.is_some() {
            let action = (key.code == KeyCode::Esc)
                .then_some(Action::Navigation(NavigationAction::CloseTrackDetails));
            send_if_some(action_tx, action).await;
            return true;
        }
        // The notification log is modal; any documented dismiss key closes it.
        if self.state.show_notification_log {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('!') | KeyCode::Char('q') | KeyCode::Enter
            ) {
                self.state.show_notification_log = false;
            }
            return true;
        }
        if self.state.search_detail_open && matches!(key.code, KeyCode::Esc | KeyCode::Char('i')) {
            let _ = action_tx
                .send(Action::Navigation(NavigationAction::ToggleSearchDetail))
                .await;
            return true;
        }
        // Esc always cancels an import modal; Enter confirms only a successfully
        // fetched review, never a loading or failed import.
        if let Some(import) = &self.state.import {
            let action = match (import, key.code) {
                (ImportState::Review { .. }, KeyCode::Enter) => {
                    Some(Action::Playlists(PlaylistAction::ConfirmImport))
                }
                (_, KeyCode::Esc) => Some(Action::Playlists(PlaylistAction::CancelImport)),
                _ => None,
            };
            send_if_some(action_tx, action).await;
            return true;
        }
        // Confirmation dialogs accept only explicit yes/no input.
        if self.state.confirm.is_some() {
            let action = match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    Some(Action::Playlists(PlaylistAction::ConfirmYes))
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    Some(Action::Playlists(PlaylistAction::ConfirmNo))
                }
                _ => None,
            };
            send_if_some(action_tx, action).await;
            return true;
        }
        if self.state.playlist_editor.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::Playlists(PlaylistAction::PlaylistEditorSubmit)),
                KeyCode::Esc => Some(Action::Playlists(PlaylistAction::PlaylistEditorCancel)),
                KeyCode::Tab | KeyCode::BackTab => {
                    Some(Action::Playlists(PlaylistAction::PlaylistEditorNextField))
                }
                KeyCode::Backspace => {
                    Some(Action::Playlists(PlaylistAction::PlaylistEditorBackspace))
                }
                KeyCode::Char(character) => Some(Action::Playlists(
                    PlaylistAction::PlaylistEditorInput(character),
                )),
                _ => None,
            };
            send_if_some(action_tx, action).await;
            return true;
        }
        // Text prompt modal.
        if self.state.prompt.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::Playlists(PlaylistAction::PromptSubmit)),
                KeyCode::Esc => Some(Action::Playlists(PlaylistAction::PromptCancel)),
                KeyCode::Backspace => Some(Action::Playlists(PlaylistAction::PromptBackspace)),
                KeyCode::Char(character) => {
                    Some(Action::Playlists(PlaylistAction::PromptInput(character)))
                }
                _ => None,
            };
            send_if_some(action_tx, action).await;
            return true;
        }
        // Add-to-playlist picker modal.
        if self.state.picker.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::Playlists(PlaylistAction::PickerSubmit)),
                KeyCode::Esc => Some(Action::Playlists(PlaylistAction::PickerCancel)),
                KeyCode::Backspace => Some(Action::Playlists(PlaylistAction::PickerBackspace)),
                KeyCode::Down => Some(Action::Playlists(PlaylistAction::PickerNext)),
                KeyCode::Up => Some(Action::Playlists(PlaylistAction::PickerPrevious)),
                KeyCode::Char(character) => {
                    Some(Action::Playlists(PlaylistAction::PickerInput(character)))
                }
                _ => None,
            };
            send_if_some(action_tx, action).await;
            return true;
        }
        false
    }
}

async fn send_if_some(action_tx: &mpsc::Sender<Action>, action: Option<Action>) {
    if let Some(action) = action {
        let _ = action_tx.send(action).await;
    }
}
