//! Playback-event bookkeeping, history outcomes, and resume snapshots.

use std::time::{Duration, Instant};

use crate::app::App;
use crate::history::model::{HistoryEntry, PlaybackOutcome};
use crate::playback::PlaybackEvent;

impl App {
    /// Persist a session snapshot, throttled to one write per interval.
    pub(in crate::app) fn maybe_save_session(&mut self, position_seconds: f64, force: bool) {
        const SESSION_SAVE_INTERVAL: Duration = Duration::from_secs(5);
        let due = self
            .last_session_save
            .is_none_or(|saved| saved.elapsed() >= SESSION_SAVE_INTERVAL);
        if !force && !due {
            return;
        }
        self.last_session_save = Some(Instant::now());
        let mut document = crate::persistence::session::SessionDocument::new(
            self.state.domain.current_track.clone(),
            position_seconds,
            self.state.domain.playback.volume,
        );
        document.activity = self.state.domain.activity.clone();
        document.resume_points = self.state.domain.resume_points.clone();
        let path = self.paths.session_file();
        self.submit_persistence("session", "session", move || {
            crate::persistence::session::save(&path, &document)
        });
    }

    /// Track playback lifecycle for history recording and resume-session snapshots.
    pub(in crate::app) fn on_playback_event(&mut self, event: &PlaybackEvent) {
        match event {
            PlaybackEvent::Started => {
                self.listening.started();
                self.maybe_save_session(0.0, true);
            }
            PlaybackEvent::PositionChanged(position) => {
                self.listening.position(*position);
                self.maybe_save_session(*position, false);
            }
            PlaybackEvent::PauseChanged(paused) => {
                self.listening.paused(*paused);
                if *paused {
                    self.capture_resume_point();
                }
            }
            PlaybackEvent::EndFile { reason } => {
                self.capture_resume_point();
                let outcome = if reason == "error" {
                    PlaybackOutcome::Failed
                } else {
                    PlaybackOutcome::Completed
                };
                self.record_current(outcome);
            }
            _ => {}
        }
    }

    /// Append the outgoing track to history with the supplied outcome.
    pub(in crate::app) fn record_current(&mut self, outcome: PlaybackOutcome) {
        let Some(track) = self.state.domain.current_track.clone() else {
            let _ = self.listening.finish();
            return;
        };
        let listened = self.listening.finish();
        let Some(history) = self.history.as_mut() else {
            return;
        };
        history.record(HistoryEntry::from_track(&track, None, outcome, listened));
        self.list_revision = self.list_revision.wrapping_add(1);
        self.filter_sync_key = None;
        self.persist_history();
    }

    /// Capture a durable per-track resume point when playback is in its resumable range.
    pub(in crate::app) fn capture_resume_point(&mut self) {
        let Some(track) = &self.state.domain.current_track else {
            return;
        };
        let Some(duration) = self
            .state
            .domain
            .playback
            .duration_seconds
            .or_else(|| track.duration_seconds.map(|value| value as f64))
        else {
            return;
        };
        if self.state.domain.resume_points.record(
            track.id.clone(),
            self.state.domain.playback.position_seconds,
            duration,
            chrono::Utc::now(),
        ) {
            self.maybe_save_session(self.state.domain.playback.position_seconds, true);
        }
    }
}
