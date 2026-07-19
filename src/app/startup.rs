//! Session restoration and CLI startup intent handling.

use tokio::sync::mpsc;

use crate::app::action::Action;
use crate::app::state::View;
use crate::app::{App, StartupIntent};

impl App {
    /// Restore the previous session and apply any CLI startup intent once,
    /// immediately after mpv startup.
    pub(super) async fn init_session(&mut self, action_tx: &mpsc::Sender<Action>) {
        use crate::config::ResumeMode;

        let mode = match self.startup_intent {
            Some(StartupIntent::Resume) => ResumeMode::Playing,
            _ => self.config.playback.resume_on_launch,
        };
        let session = crate::persistence::session::load(&self.paths.session_file());
        if let Some(document) = &session {
            self.state.activity = document.activity.clone();
            self.state.resume_points = document.resume_points.clone();
        }

        if let Some(StartupIntent::PlayQuery(query)) = self.startup_intent.clone() {
            self.state.view = View::Search;
            self.autoplay_first_search = true;
            self.submit_text_query(query, action_tx).await;
            return;
        }

        if mode == ResumeMode::Off || self.playback.is_none() {
            return;
        }
        let Some(doc) = session else {
            return;
        };
        let Some(track) = doc.track else {
            return;
        };

        if let Some(playback) = self.playback.as_mut()
            && doc.volume > 0
            && doc.volume != self.config.playback.default_volume
        {
            let _ = playback.set_volume(doc.volume).await;
        }

        if let Some(position) = self
            .state
            .queue
            .order
            .iter()
            .position(|&index| self.state.queue.tracks[index].id == track.id)
        {
            // Align the restored queue cursor with the session track so
            // next and previous continue from the expected position.
            self.state.queue.position = Some(position);
        }

        self.state.current_track = Some(track.clone());
        self.state.playback.position_seconds = doc.position_seconds;
        self.state.playback.duration_seconds =
            track.duration_seconds.map(|duration| duration as f64);
        self.state.pending_resume = Some(crate::app::state::PendingResume {
            track: track.clone(),
            position_seconds: doc.position_seconds,
            armed: false,
            play_on_load: mode == ResumeMode::Playing,
        });

        self.spawn_pending_resume_resolution(track, action_tx);
    }
}
