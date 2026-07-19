//! Add-to-playlist picker submission workflow.

use crate::app::App;
use crate::playlists::Playlist;

impl App {
    /// Add the picked track to an existing or newly named playlist.
    pub(super) async fn submit_picker(&mut self) {
        let Some(picker) = self.state.picker.take() else {
            return;
        };
        let (create_new, matching) =
            crate::app::filter::picker_candidates(&self.state.playlists, &picker.filter);

        let target_index = if create_new {
            if picker.selected == 0 {
                let playlist = Playlist::new(picker.filter.trim());
                match self.playlists.save(&playlist) {
                    Ok(()) => {
                        self.state.playlists.push(playlist);
                        Some(self.state.playlists.len() - 1)
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
        let Some(playlist) = self.state.playlists.get_mut(target_index) else {
            return;
        };

        if playlist
            .tracks
            .iter()
            .any(|track| track.id == picker.track.id)
        {
            let name = playlist.name.clone();
            self.state.notify(&format!("Already in \"{name}\""), false);
            return;
        }
        playlist
            .tracks
            .push(crate::playlists::model::PlaylistTrack::from(&picker.track));
        playlist.updated_at = chrono::Utc::now();
        let snapshot = playlist.clone();
        match self.playlists.save(&snapshot) {
            Ok(()) => {
                self.state
                    .activity
                    .push(crate::history::activity::ActivityEvent::new(
                        crate::history::activity::ActivityKind::AddedToPlaylist,
                        picker.track.title.clone(),
                        snapshot.name.clone(),
                    ));
                self.state
                    .notify(&format!("Added to \"{}\"", snapshot.name), false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
        self.state.sort_playlists_by_updated();
        self.maybe_save_session(self.state.playback.position_seconds, true);
    }
}
