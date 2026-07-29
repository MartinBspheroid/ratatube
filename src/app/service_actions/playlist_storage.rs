//! Playlist loading, deletion, prompts, and import confirmation.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::{ImportState, PromptPurpose};

impl App {
    /// Ask for confirmation before deleting the selected playlist.
    pub(super) async fn confirm_delete_selected_playlist(
        &mut self,
        action_tx: &mpsc::Sender<Action>,
    ) {
        if let Some(id) = self.selected_playlist_id() {
            let _ = action_tx
                .send(Action::Playlists(PlaylistAction::DeletePlaylist(id)))
                .await;
        }
    }

    /// Replace the queue with a stored playlist and start its first track.
    pub(super) async fn load_playlist_into_queue(
        &mut self,
        id: &str,
        action_tx: &mpsc::Sender<Action>,
    ) {
        let Some(tracks) = self.playlist_tracks(id) else {
            return;
        };
        self.state.domain.queue.load_tracks(tracks);
        self.state.bump_queue_revision();
        if self.state.domain.queue.order.is_empty() {
            return;
        }
        self.state.domain.queue.position = Some(0);
        self.state.domain.current_track = self.state.domain.queue.current().cloned();
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

    /// Append a stored playlist to the end of the queue.
    pub(super) async fn append_playlist_to_queue(
        &mut self,
        id: &str,
        action_tx: &mpsc::Sender<Action>,
    ) {
        let Some(tracks) = self.playlist_tracks(id) else {
            return;
        };
        let changed = !tracks.is_empty();
        for track in tracks {
            self.state.domain.queue.push(track);
        }
        if changed {
            self.state.bump_queue_revision();
        }
        self.state.notify("Playlist appended to queue", false);
        self.execute(vec![Effect::PersistQueue], action_tx).await;
    }

    /// Parse pasted playlist JSON and persist every accepted playlist.
    pub(super) fn import_playlists_json(&mut self, json: &str) {
        match crate::playlists::import::parse_pasted_json(json) {
            Ok(playlists) => self.save_json_playlists(playlists),
            Err(message) => self.state.notify(&message, true),
        }
    }

    /// Delete a stored playlist and drop it from the catalog.
    pub(super) fn delete_playlist(&mut self, id: &str) {
        match self.playlists.delete(id) {
            Ok(()) => {
                let previous_len = self.state.domain.playlists.len();
                self.state
                    .domain
                    .playlists
                    .retain(|playlist| playlist.id != id);
                if self.state.domain.playlists.len() != previous_len {
                    self.state.bump_playlists_revision();
                }
                self.state.ui.selected_playlist = None;
                self.state.clamp_selection();
                self.state.notify("Playlist deleted", false);
            }
            Err(err) => self.state.notify(&format!("Delete failed: {err}"), true),
        }
    }

    /// Re-dispatch the action a confirmation modal was guarding.
    pub(super) async fn dispatch_confirmed_action(&mut self, action_tx: &mpsc::Sender<Action>) {
        if let Some(confirm) = self.state.ui.confirm.take() {
            let action = *confirm.action;
            let _ = action_tx.send(action).await;
        }
    }

    /// Apply the open prompt's purpose to its trimmed buffer.
    pub(super) async fn submit_prompt(&mut self, action_tx: &mpsc::Sender<Action>) {
        let Some(prompt) = self.state.ui.prompt.take() else {
            return;
        };
        let text = prompt.buffer.trim().to_string();
        match (prompt.purpose, text.is_empty()) {
            (purpose, true) => {
                let required = match purpose {
                    PromptPurpose::ImportPlaylistUrl => "A URL is required",
                    PromptPurpose::ImportPlaylistJson => "Playlist JSON is required",
                    PromptPurpose::SaveQueueAsPlaylist
                    | PromptPurpose::NewPlaylist
                    | PromptPurpose::RenamePlaylist => "A playlist name is required",
                };
                self.state.notify(required, true);
            }
            (PromptPurpose::SaveQueueAsPlaylist, false) => self.save_queue_as_playlist(&text),
            (PromptPurpose::NewPlaylist, false) => self.create_playlist(&text),
            (PromptPurpose::RenamePlaylist, false) => self.rename_selected_playlist(&text),
            (PromptPurpose::ImportPlaylistUrl, false) => {
                let _ = action_tx
                    .send(Action::Playlists(PlaylistAction::StartImport(text)))
                    .await;
            }
            (PromptPurpose::ImportPlaylistJson, false) => {
                match crate::playlists::import::parse_pasted_json(&text) {
                    Ok(playlists) => self.save_json_playlists(playlists),
                    Err(message) => {
                        self.state.ui.prompt = Some(crate::app::state::PromptState {
                            purpose: PromptPurpose::ImportPlaylistJson,
                            buffer: text,
                        });
                        self.state.notify(&message, true);
                    }
                }
            }
        }
    }

    /// Persist the reviewed import and announce it as an activity event.
    pub(super) async fn confirm_import(&mut self, action_tx: &mpsc::Sender<Action>) {
        let Some(ImportState::Review { playlist, .. }) = self.state.domain.import.take() else {
            return;
        };
        match self.playlists.save(&playlist) {
            Ok(()) => {
                let saved = *playlist;
                self.state
                    .domain
                    .activity
                    .push(crate::history::activity::ActivityEvent::new(
                        crate::history::activity::ActivityKind::PlaylistImported,
                        saved.name.clone(),
                        format!("{} tracks", saved.tracks.len()),
                    ));
                self.maybe_save_session(self.state.domain.playback.position_seconds, true);
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
