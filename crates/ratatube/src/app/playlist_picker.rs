//! Add-to-playlist picker submission workflow.

use crate::app::App;
use crate::playlists::Playlist;

impl App {
    /// Apply the picker by adding to an existing playlist or creating one
    /// named after the filter text first.
    pub(super) async fn submit_picker(&mut self) {
        let Some(picker) = self.state.ui.picker.take() else {
            return;
        };
        let (create_new, matching) =
            crate::app::filter::picker_candidates(&self.state.domain.playlists, &picker.filter);

        let target_index = if create_new {
            if picker.selected == 0 {
                let playlist = Playlist::new(picker.filter.trim());
                match self.playlists.save(&playlist) {
                    Ok(()) => {
                        self.state.domain.playlists.push(playlist);
                        self.state.bump_playlists_revision();
                        Some(self.state.domain.playlists.len() - 1)
                    }
                    Err(err) => {
                        self.state.notify(&format!("Save failed: {err}"), true);
                        return;
                    }
                }
            } else {
                matching.get(picker.selected - 1).copied()
            }
        } else {
            matching.get(picker.selected).copied()
        };
        let Some(target_index) = target_index else {
            return;
        };
        self.add_track_to_playlist_at(target_index, picker.track);
    }

    /// Add `track` to the stored playlist at `target_index`, with dedupe,
    /// durable save, activity, and notification.
    pub(super) fn add_track_to_playlist_at(
        &mut self,
        target_index: usize,
        track: crate::media::Track,
    ) {
        let Some(playlist) = self.state.domain.playlists.get_mut(target_index) else {
            return;
        };

        if playlist
            .tracks
            .iter()
            .any(|existing| existing.id == track.id)
        {
            let name = playlist.name.clone();
            self.state.notify(&format!("Already in \"{name}\""), false);
            return;
        }
        playlist
            .tracks
            .push(crate::playlists::model::PlaylistTrack::from(&track));
        playlist.updated_at = chrono::Utc::now();
        let snapshot = playlist.clone();
        self.state.bump_playlists_revision();
        match self.playlists.save(&snapshot) {
            Ok(()) => {
                self.state
                    .domain
                    .activity
                    .push(crate::history::activity::ActivityEvent::new(
                        crate::history::activity::ActivityKind::AddedToPlaylist,
                        track.title.clone(),
                        snapshot.name.clone(),
                    ));
                self.state
                    .notify(&format!("Added to \"{}\"", snapshot.name), false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
        self.state.sort_playlists_by_updated();
        self.maybe_save_session(self.state.domain.playback.position_seconds, true);
    }
}
