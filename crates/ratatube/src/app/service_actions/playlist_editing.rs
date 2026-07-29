//! Picker, metadata editor, removal, and reordering workflows.

use crate::app::App;
use crate::app::state::View;
use crate::media::Track;

impl App {
    /// Open the picker for the track selected in the active view.
    pub(super) fn open_playlist_picker(&mut self) {
        if let Some(track) = self.resolve_selected_track() {
            self.state.ui.picker = Some(crate::app::state::PickerState {
                track,
                filter: String::new(),
                selected: 0,
            });
        }
    }

    /// Remove the selected track from the open playlist detail view.
    pub(super) fn remove_selected_from_playlist(&mut self) {
        if self.state.ui.view != View::PlaylistDetail {
            return;
        }
        let index = self.state.resolve_index(self.state.ui.selected_index);
        let Some(playlist) = self
            .state
            .ui
            .selected_playlist
            .and_then(|playlist_index| self.state.domain.playlists.get_mut(playlist_index))
        else {
            return;
        };
        if index >= playlist.tracks.len() {
            return;
        }
        playlist.tracks.remove(index);
        playlist.updated_at = chrono::Utc::now();
        let snapshot = playlist.clone();
        self.state.bump_playlists_revision();
        match self.playlists.save(&snapshot) {
            Ok(()) => self.state.notify("Removed from playlist", false),
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
        self.state.clamp_selection();
    }

    /// Remove one captured playlist occurrence if its track still matches.
    pub(super) fn remove_playlist_occurrence(
        &mut self,
        playlist_id: &str,
        track_index: usize,
        expected_track: &Track,
        expected_revision: u64,
    ) {
        if self.state.domain.playlists_revision != expected_revision {
            self.state
                .notify("Playlist changed; removal cancelled", true);
            return;
        }
        let Some(playlist_index) = self
            .state
            .domain
            .playlists
            .iter()
            .position(|playlist| playlist.id == playlist_id)
        else {
            self.state
                .notify("Playlist changed; removal cancelled", true);
            return;
        };
        let still_matches = self.state.domain.playlists[playlist_index]
            .tracks
            .get(track_index)
            .map(Track::from)
            .as_ref()
            == Some(expected_track);
        if !still_matches {
            self.state
                .notify("Playlist changed; removal cancelled", true);
            return;
        }
        let previous = self.state.domain.playlists[playlist_index].clone();
        let playlist = &mut self.state.domain.playlists[playlist_index];
        playlist.tracks.remove(track_index);
        playlist.updated_at = chrono::Utc::now();
        let snapshot = playlist.clone();
        self.state.bump_playlists_revision();
        match self.playlists.save(&snapshot) {
            Ok(()) => self.state.notify("Removed from playlist", false),
            Err(error) => {
                self.state.domain.playlists[playlist_index] = previous;
                self.state.notify(&format!("Save failed: {error}"), true);
            }
        }
        self.state.clamp_selection();
    }

    /// Persist the open metadata editor onto the selected playlist.
    pub(super) fn submit_playlist_editor(&mut self) {
        let Some(editor) = self.state.ui.playlist_editor.clone() else {
            return;
        };
        let name = editor.name.trim();
        if name.is_empty() {
            self.state.notify("Playlist name is required", true);
            return;
        }
        let Some(playlist) = self
            .state
            .ui
            .selected_playlist
            .and_then(|index| self.state.domain.playlists.get_mut(index))
        else {
            self.state.ui.playlist_editor = None;
            return;
        };
        let previous = playlist.clone();
        playlist.name = name.to_string();
        playlist.description = editor.description.trim().to_string();
        playlist.updated_at = chrono::Utc::now();
        match self.playlists.save(playlist) {
            Ok(()) => {
                self.state.ui.playlist_editor = None;
                self.state.notify("Playlist details saved", false);
            }
            Err(error) => {
                *playlist = previous;
                self.state.notify(&format!("Save failed: {error}"), true);
            }
        }
    }

    /// Add a track to an existing stored playlist by id.
    pub(super) fn add_track_to_playlist(&mut self, playlist_id: &str, track: Track) {
        let Some(index) = self
            .state
            .domain
            .playlists
            .iter()
            .position(|playlist| playlist.id == playlist_id)
        else {
            self.state.notify("Playlist no longer exists", true);
            return;
        };
        self.add_track_to_playlist_at(index, track);
    }

    /// Create a playlist and add the track to it.
    pub(super) fn add_track_to_new_playlist(&mut self, name: &str, track: Track) {
        let playlist = crate::playlists::Playlist::new(name.trim());
        match self.playlists.save(&playlist) {
            Ok(()) => {
                self.state.domain.playlists.push(playlist);
                self.state.bump_playlists_revision();
                let index = self.state.domain.playlists.len() - 1;
                self.add_track_to_playlist_at(index, track);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
    }

    /// Reorder a stored playlist's tracks by explicit positions.
    pub(super) fn move_track_in_playlist(&mut self, id: &str, from: usize, to: usize) {
        let Some(playlist) = self
            .state
            .domain
            .playlists
            .iter_mut()
            .find(|playlist| playlist.id == id)
        else {
            return;
        };
        let len = playlist.tracks.len();
        if len < 2 || from >= len || to >= len || from == to {
            return;
        }
        let track = playlist.tracks.remove(from);
        playlist.tracks.insert(to, track);
        playlist.updated_at = chrono::Utc::now();
        let snapshot = playlist.clone();
        self.state.bump_playlists_revision();
        if let Err(err) = self.playlists.save(&snapshot) {
            self.state.notify(&format!("Save failed: {err}"), true);
        }
    }

    /// Move the selected playlist track up (-1) or down (+1).
    pub(super) fn move_selected_in_playlist(&mut self, delta: i32) {
        if self.state.ui.view != View::PlaylistDetail {
            return;
        }
        if self.state.ui.visible_indices.is_some() {
            self.state
                .notify("Clear the filter (Esc) to reorder", false);
            return;
        }
        let from = self.state.ui.selected_index;
        let Some(playlist) = self
            .state
            .ui
            .selected_playlist
            .and_then(|index| self.state.domain.playlists.get_mut(index))
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
        self.state.bump_playlists_revision();
        self.state.ui.selected_index = to;
        if let Err(err) = self.playlists.save(&snapshot) {
            self.state.notify(&format!("Save failed: {err}"), true);
        }
    }

    /// Update a stored playlist's name (and optionally description) by id,
    /// rolling back on a failed save.
    pub(super) fn edit_playlist_by_id(&mut self, id: &str, name: &str, description: Option<&str>) {
        let name = name.trim();
        if name.is_empty() {
            self.state.notify("Playlist name is required", true);
            return;
        }
        let Some(playlist) = self
            .state
            .domain
            .playlists
            .iter_mut()
            .find(|playlist| playlist.id == id)
        else {
            self.state.notify("Playlist no longer exists", true);
            return;
        };
        let previous = playlist.clone();
        playlist.name = name.to_string();
        if let Some(description) = description {
            playlist.description = description.trim().to_string();
        }
        playlist.updated_at = chrono::Utc::now();
        let snapshot = playlist.clone();
        self.state.bump_playlists_revision();
        match self.playlists.save(&snapshot) {
            Ok(()) => self.state.notify("Playlist details saved", false),
            Err(error) => {
                if let Some(playlist) = self
                    .state
                    .domain
                    .playlists
                    .iter_mut()
                    .find(|playlist| playlist.id == id)
                {
                    *playlist = previous;
                }
                self.state.notify(&format!("Save failed: {error}"), true);
            }
        }
        self.state.sort_playlists_by_updated();
    }
}
