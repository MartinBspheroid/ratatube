//! Playlist selection and persistence helpers.

use crate::app::App;
use crate::app::state::View;
use crate::media::Track;
use crate::playlists::Playlist;

impl App {
    /// Resolve the playlist ID relevant to the current view and selection.
    pub(super) fn selected_playlist_id(&self) -> Option<String> {
        match self.state.view {
            View::Home if self.state.home_section == crate::app::state::HomeSection::Playlists => {
                self.state
                    .playlists
                    .get(self.state.selected_index)
                    .map(|playlist| playlist.id.clone())
            }
            View::Playlists => self
                .state
                .playlists
                .get(self.state.selected_index)
                .map(|playlist| playlist.id.clone()),
            View::PlaylistDetail => self
                .state
                .selected_playlist
                .and_then(|index| self.state.playlists.get(index))
                .map(|playlist| playlist.id.clone()),
            _ => None,
        }
    }

    /// Clone the tracks belonging to a stored playlist ID.
    pub(super) fn playlist_tracks(&self, id: &str) -> Option<Vec<Track>> {
        self.state
            .playlists
            .iter()
            .find(|playlist| playlist.id == id)
            .map(Playlist::to_tracks)
    }

    /// Save the current queue as a named playlist and refresh presentation order.
    pub(super) fn save_queue_as_playlist(&mut self, name: &str) {
        if self.state.queue.tracks.is_empty() {
            self.state.notify("Queue is empty", true);
            return;
        }
        let mut playlist = Playlist::new(name);
        playlist.tracks = self
            .state
            .queue
            .tracks
            .iter()
            .map(crate::playlists::model::PlaylistTrack::from)
            .collect();
        match self.playlists.save(&playlist) {
            Ok(()) => {
                self.state.playlists.push(playlist);
                self.state.bump_playlists_revision();
                self.state.sort_playlists_by_updated();
                self.state.notify("Queue saved as playlist", false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
    }

    /// Create and persist an empty playlist.
    pub(super) fn create_playlist(&mut self, name: &str) {
        let playlist = Playlist::new(name);
        match self.playlists.save(&playlist) {
            Ok(()) => {
                self.state.playlists.push(playlist);
                self.state.bump_playlists_revision();
                self.state.sort_playlists_by_updated();
                self.state.notify("Playlist created", false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
    }

    /// Validate and persist every playlist from a pasted JSON import atomically.
    pub(super) fn save_json_playlists(&mut self, playlists: Vec<Playlist>) {
        let requested_playlist_count = playlists.len();
        let requested_track_count = playlists
            .iter()
            .map(|playlist| playlist.tracks.len())
            .sum::<usize>();
        let mut staged: Vec<Playlist> = Vec::with_capacity(requested_playlist_count);
        let mut added_tracks = 0usize;
        for incoming in playlists {
            let matching_staged = staged.iter().position(|playlist| {
                playlist
                    .name
                    .trim()
                    .eq_ignore_ascii_case(incoming.name.trim())
            });
            let matching_existing = self.state.playlists.iter().find(|playlist| {
                playlist
                    .name
                    .trim()
                    .eq_ignore_ascii_case(incoming.name.trim())
            });
            if let Some(index) = matching_staged {
                added_tracks += merge_playlist_tracks(&mut staged[index], incoming);
            } else if let Some(existing) = matching_existing {
                let mut merged = existing.clone();
                added_tracks += merge_playlist_tracks(&mut merged, incoming);
                staged.push(merged);
            } else {
                added_tracks += incoming.tracks.len();
                staged.push(incoming);
            }
        }
        let originals = staged
            .iter()
            .filter_map(|staged_playlist| {
                self.state
                    .playlists
                    .iter()
                    .find(|playlist| playlist.id == staged_playlist.id)
                    .cloned()
            })
            .collect::<Vec<_>>();
        let mut saved_ids: Vec<String> = Vec::with_capacity(staged.len());
        for playlist in &staged {
            if let Err(error) = self.playlists.save(playlist) {
                for id in &saved_ids {
                    let rollback = originals
                        .iter()
                        .find(|playlist| playlist.id == *id)
                        .map_or_else(
                            || self.playlists.delete(id),
                            |playlist| self.playlists.save(playlist),
                        );
                    if let Err(rollback_error) = rollback {
                        tracing::error!(?rollback_error, playlist_id = %id, "JSON import rollback failed");
                    }
                }
                self.state.notify(
                    &format!("JSON import failed; no playlists kept: {error}"),
                    true,
                );
                return;
            }
            saved_ids.push(playlist.id.clone());
        }
        let changed = !staged.is_empty();
        for playlist in staged {
            match self
                .state
                .playlists
                .iter()
                .position(|existing| existing.id == playlist.id)
            {
                Some(index) => self.state.playlists[index] = playlist,
                None => self.state.playlists.push(playlist),
            }
        }
        if changed {
            self.state.bump_playlists_revision();
        }
        self.state.sort_playlists_by_updated();
        self.state
            .activity
            .push(crate::history::activity::ActivityEvent::new(
                crate::history::activity::ActivityKind::PlaylistImported,
                format!("{requested_playlist_count} JSON playlists"),
                format!("{added_tracks} new tracks"),
            ));
        self.maybe_save_session(self.state.playback.position_seconds, true);
        self.state.notify(
            &format!(
                "Imported {added_tracks} new tracks · {} duplicates skipped",
                requested_track_count.saturating_sub(added_tracks)
            ),
            false,
        );
    }

    /// Rename the selected playlist and persist the updated catalog.
    pub(super) fn rename_selected_playlist(&mut self, name: &str) {
        let Some(id) = self.selected_playlist_id() else {
            return;
        };
        let Some(playlist) = self
            .state
            .playlists
            .iter_mut()
            .find(|playlist| playlist.id == id)
        else {
            return;
        };
        playlist.name = name.to_string();
        playlist.updated_at = chrono::Utc::now();
        match self.playlists.save(playlist) {
            Ok(()) => self.state.notify("Playlist renamed", false),
            Err(err) => self.state.notify(&format!("Rename failed: {err}"), true),
        }
        self.state.sort_playlists_by_updated();
    }
}

fn merge_playlist_tracks(target: &mut Playlist, incoming: Playlist) -> usize {
    let mut ids = target
        .tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let before = target.tracks.len();
    target.tracks.extend(
        incoming
            .tracks
            .into_iter()
            .filter(|track| ids.insert(track.id.clone())),
    );
    if target.description.is_empty() && !incoming.description.is_empty() {
        target.description = incoming.description;
    }
    let added = target.tracks.len() - before;
    if added > 0 {
        target.updated_at = chrono::Utc::now();
    }
    added
}
