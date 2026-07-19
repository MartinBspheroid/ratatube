//! Picker, metadata editor, removal, and reordering workflows.

use crate::app::App;
use crate::app::action::PlaylistAction;
use crate::app::state::View;

impl App {
    /// Apply picker and playlist-editor workflows that require stored playlists.
    pub(super) async fn handle_playlist_editing(&mut self, action: PlaylistAction) {
        match action {
            PlaylistAction::OpenPlaylistPicker => {
                if let Some(track) = self.resolve_selected_track() {
                    self.state.picker = Some(crate::app::state::PickerState {
                        track,
                        filter: String::new(),
                        selected: 0,
                    });
                }
            }
            PlaylistAction::OpenPlaylistPickerForTrack(track) => {
                self.state.picker = Some(crate::app::state::PickerState {
                    track,
                    filter: String::new(),
                    selected: 0,
                });
            }
            PlaylistAction::PickerSubmit => self.submit_picker().await,
            PlaylistAction::RemoveSelectedFromPlaylist => {
                if self.state.view != View::PlaylistDetail {
                    return;
                }
                let index = self.state.resolve_index(self.state.selected_index);
                let Some(playlist) = self
                    .state
                    .selected_playlist
                    .and_then(|playlist_index| self.state.playlists.get_mut(playlist_index))
                else {
                    return;
                };
                if index >= playlist.tracks.len() {
                    return;
                }
                playlist.tracks.remove(index);
                playlist.updated_at = chrono::Utc::now();
                let snapshot = playlist.clone();
                match self.playlists.save(&snapshot) {
                    Ok(()) => self.state.notify("Removed from playlist", false),
                    Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
                }
                self.state.clamp_selection();
            }
            PlaylistAction::RemoveTrackOccurrence {
                playlist_id,
                track_index,
                expected_track,
            } => {
                let Some(playlist_index) = self
                    .state
                    .playlists
                    .iter()
                    .position(|playlist| playlist.id == playlist_id)
                else {
                    self.state
                        .notify("Playlist changed; removal cancelled", true);
                    return;
                };
                let still_matches = self.state.playlists[playlist_index]
                    .tracks
                    .get(track_index)
                    .map(crate::media::Track::from)
                    .as_ref()
                    == Some(&expected_track);
                if !still_matches {
                    self.state
                        .notify("Playlist changed; removal cancelled", true);
                    return;
                }
                let previous = self.state.playlists[playlist_index].clone();
                let playlist = &mut self.state.playlists[playlist_index];
                playlist.tracks.remove(track_index);
                playlist.updated_at = chrono::Utc::now();
                let snapshot = playlist.clone();
                match self.playlists.save(&snapshot) {
                    Ok(()) => self.state.notify("Removed from playlist", false),
                    Err(error) => {
                        self.state.playlists[playlist_index] = previous;
                        self.state.notify(&format!("Save failed: {error}"), true);
                    }
                }
                self.state.clamp_selection();
            }
            PlaylistAction::PlaylistEditorSubmit => {
                let Some(editor) = self.state.playlist_editor.clone() else {
                    return;
                };
                let name = editor.name.trim();
                if name.is_empty() {
                    self.state.notify("Playlist name is required", true);
                    return;
                }
                let Some(playlist) = self
                    .state
                    .selected_playlist
                    .and_then(|index| self.state.playlists.get_mut(index))
                else {
                    self.state.playlist_editor = None;
                    return;
                };
                let previous = playlist.clone();
                playlist.name = name.to_string();
                playlist.description = editor.description.trim().to_string();
                playlist.updated_at = chrono::Utc::now();
                match self.playlists.save(playlist) {
                    Ok(()) => {
                        self.state.playlist_editor = None;
                        self.state.notify("Playlist details saved", false);
                    }
                    Err(error) => {
                        *playlist = previous;
                        self.state.notify(&format!("Save failed: {error}"), true);
                    }
                }
            }
            PlaylistAction::MoveSelectedInPlaylist(delta) => {
                if self.state.view != View::PlaylistDetail {
                    return;
                }
                if self.state.visible_indices.is_some() {
                    self.state
                        .notify("Clear the filter (Esc) to reorder", false);
                    return;
                }
                let from = self.state.selected_index;
                let Some(playlist) = self
                    .state
                    .selected_playlist
                    .and_then(|index| self.state.playlists.get_mut(index))
                else {
                    return;
                };
                let len = playlist.tracks.len();
                if len < 2 || from >= len {
                    return;
                }
                let to = from.saturating_add_signed(delta as isize).min(len - 1);
                if from == to {
                    return;
                }
                let track = playlist.tracks.remove(from);
                playlist.tracks.insert(to, track);
                playlist.updated_at = chrono::Utc::now();
                let snapshot = playlist.clone();
                self.state.selected_index = to;
                if let Err(err) = self.playlists.save(&snapshot) {
                    self.state.notify(&format!("Save failed: {err}"), true);
                }
            }
            _ => {}
        }
    }
}
