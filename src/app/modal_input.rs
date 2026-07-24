//! Modal-first keyboard routing.

use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction, PlaylistAction};
use crate::app::state::{ImportState, ModalCapture};

impl App {
    /// Route one modal-owned key and report whether an overlay consumed input.
    pub(super) async fn handle_modal_key(
        &mut self,
        capture: ModalCapture,
        key: &KeyEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match capture {
            ModalCapture::TrackContext => {
                let action = match key.code {
                    KeyCode::Esc => Some(Action::Navigation(NavigationAction::CloseTrackContext)),
                    KeyCode::Char('j') | KeyCode::Down => {
                        Some(Action::Navigation(NavigationAction::MoveTrackContext(1)))
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        Some(Action::Navigation(NavigationAction::MoveTrackContext(-1)))
                    }
                    KeyCode::Enter => {
                        Some(Action::Navigation(NavigationAction::SubmitTrackContext))
                    }
                    _ => None,
                };
                send_if_some(action_tx, action).await;
            }
            ModalCapture::TrackDetails => {
                let action = (key.code == KeyCode::Esc)
                    .then_some(Action::Navigation(NavigationAction::CloseTrackDetails));
                send_if_some(action_tx, action).await;
            }
            ModalCapture::NotificationLog => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('!') | KeyCode::Char('q') | KeyCode::Enter
                ) {
                    self.state.ui.show_notification_log = false;
                }
            }
            ModalCapture::Settings => {
                let ctrl_p = key.code == KeyCode::Char('p')
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL);
                let action = if ctrl_p {
                    // ctrl+p toggles the menu closed again.
                    Some(Action::Navigation(NavigationAction::CloseSettings))
                } else {
                    match key.code {
                        KeyCode::Esc => Some(Action::Navigation(NavigationAction::CloseSettings)),
                        KeyCode::Tab | KeyCode::BackTab => {
                            Some(Action::Navigation(NavigationAction::SettingsCycleTab))
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            Some(Action::Navigation(NavigationAction::SettingsMove(1)))
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            Some(Action::Navigation(NavigationAction::SettingsMove(-1)))
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            Some(Action::Navigation(NavigationAction::SettingsAdjust(1)))
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            Some(Action::Navigation(NavigationAction::SettingsAdjust(-1)))
                        }
                        KeyCode::Enter => {
                            Some(Action::Navigation(NavigationAction::SettingsSubmit))
                        }
                        _ => None,
                    }
                };
                send_if_some(action_tx, action).await;
            }
            ModalCapture::SearchDetails => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('i')) {
                    let _ = action_tx
                        .send(Action::Navigation(NavigationAction::ToggleSearchDetail))
                        .await;
                }
            }
            ModalCapture::Import => {
                let action = match (self.state.domain.import.as_ref(), key.code) {
                    (Some(ImportState::Review { .. }), KeyCode::Enter) => {
                        Some(Action::Playlists(PlaylistAction::ConfirmImport))
                    }
                    (Some(_), KeyCode::Esc) => {
                        Some(Action::Playlists(PlaylistAction::CancelImport))
                    }
                    _ => None,
                };
                send_if_some(action_tx, action).await;
            }
            ModalCapture::Confirm => {
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
            }
            ModalCapture::PlaylistEditor => {
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
            }
            ModalCapture::Prompt => {
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
            }
            ModalCapture::Picker => {
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
            }
        }
    }

    /// Route bracketed paste only when the topmost modal is a text prompt.
    pub(super) async fn handle_paste(&mut self, text: String, action_tx: &mpsc::Sender<Action>) {
        if self.state.modal_capture() == Some(ModalCapture::Prompt) {
            let _ = action_tx
                .send(Action::Playlists(PlaylistAction::PromptPaste(text)))
                .await;
        }
    }
}

async fn send_if_some(action_tx: &mpsc::Sender<Action>, action: Option<Action>) {
    if let Some(action) = action {
        let _ = action_tx.send(action).await;
    }
}
