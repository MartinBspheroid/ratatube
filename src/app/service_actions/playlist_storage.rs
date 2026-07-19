//! Playlist loading, deletion, prompts, and import confirmation.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::{ImportState, PromptPurpose};

impl App {
    /// Apply playlist loading, deletion, prompt, and import-persistence workflows.
    pub(super) async fn handle_playlist_storage(
        &mut self,
        action: PlaylistAction,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match action {
            PlaylistAction::DeleteSelectedPlaylist => {
                if let Some(id) = self.selected_playlist_id() {
                    let _ = action_tx
                        .send(Action::Playlists(PlaylistAction::DeletePlaylist(id)))
                        .await;
                }
            }
            PlaylistAction::LoadPlaylistIntoQueue(id) => {
                if let Some(tracks) = self.playlist_tracks(&id) {
                    self.state.queue.load_tracks(tracks);
                    if !self.state.queue.order.is_empty() {
                        self.state.queue.position = Some(0);
                        self.state.current_track = self.state.queue.current().cloned();
                        self.execute(
                            vec![
                                Effect::ResolveAndPlay {
                                    track_index_in_queue: 0,
                                },
                                Effect::PersistQueue,
                            ],
                            action_tx,
                        )
                        .await;
                    }
                }
            }
            PlaylistAction::AppendPlaylistToQueue(id) => {
                if let Some(tracks) = self.playlist_tracks(&id) {
                    for track in tracks {
                        self.state.queue.push(track);
                    }
                    self.state.notify("Playlist appended to queue", false);
                    self.execute(vec![Effect::PersistQueue], action_tx).await;
                }
            }
            PlaylistAction::DeletePlaylistConfirmed(id) => match self.playlists.delete(&id) {
                Ok(()) => {
                    self.state.playlists.retain(|playlist| playlist.id != id);
                    self.state.selected_playlist = None;
                    self.state.clamp_selection();
                    self.state.notify("Playlist deleted", false);
                }
                Err(err) => self.state.notify(&format!("Delete failed: {err}"), true),
            },
            PlaylistAction::ConfirmYes => {
                if let Some(confirm) = self.state.confirm.take() {
                    let action = *confirm.action;
                    let _ = action_tx.send(action).await;
                }
            }
            PlaylistAction::PromptSubmit => {
                if let Some(prompt) = self.state.prompt.take() {
                    let text = prompt.buffer.trim().to_string();
                    match (prompt.purpose, text.is_empty()) {
                        (purpose, true) => {
                            let required = match purpose {
                                PromptPurpose::ImportPlaylistUrl => "A URL is required",
                                PromptPurpose::ImportPlaylistJson => "Playlist JSON is required",
                                _ => "A playlist name is required",
                            };
                            self.state.notify(required, true);
                        }
                        (PromptPurpose::SaveQueueAsPlaylist, false) => {
                            self.save_queue_as_playlist(&text);
                        }
                        (PromptPurpose::NewPlaylist, false) => self.create_playlist(&text),
                        (PromptPurpose::RenamePlaylist, false) => {
                            self.rename_selected_playlist(&text);
                        }
                        (PromptPurpose::ImportPlaylistUrl, false) => {
                            let _ = action_tx
                                .send(Action::Playlists(PlaylistAction::StartImport(text)))
                                .await;
                        }
                        (PromptPurpose::ImportPlaylistJson, false) => {
                            match crate::playlists::import::parse_pasted_json(&text) {
                                Ok(playlists) => self.save_json_playlists(playlists),
                                Err(message) => {
                                    self.state.prompt = Some(crate::app::state::PromptState {
                                        purpose: PromptPurpose::ImportPlaylistJson,
                                        buffer: text,
                                    });
                                    self.state.notify(&message, true);
                                }
                            }
                        }
                    }
                }
            }
            PlaylistAction::ConfirmImport => {
                if let Some(ImportState::Review { playlist, .. }) = self.state.import.take() {
                    match self.playlists.save(&playlist) {
                        Ok(()) => {
                            let saved = *playlist;
                            self.state
                                .activity
                                .push(crate::history::activity::ActivityEvent::new(
                                    crate::history::activity::ActivityKind::PlaylistImported,
                                    saved.name.clone(),
                                    format!("{} tracks", saved.tracks.len()),
                                ));
                            self.maybe_save_session(self.state.playback.position_seconds, true);
                            self.state
                                .notify(&format!("Imported \"{}\"", saved.name), false);
                            let _ = action_tx
                                .send(Action::Playlists(PlaylistAction::PlaylistSaved(saved)))
                                .await;
                        }
                        Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
                    }
                }
            }
            _ => {}
        }
    }
}
