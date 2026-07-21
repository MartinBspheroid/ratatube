//! Playback-event snapshot bookkeeping.

use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState};
use crate::playback::PlaybackEvent;

/// Translate mpv events into follow-up effects (autoplay next on EOF, etc.).
pub(super) fn reduce_playback_event(state: &mut AppState, event: PlaybackEvent) -> Vec<Effect> {
    let status_before = state.domain.playback.status;
    state.domain.playback_status_from(&event);
    match event {
        PlaybackEvent::EndFile { ref reason } if reason == "eof" => {
            // Natural completion: advance the queue.
            super::queue::next_track(&mut state.domain)
        }
        PlaybackEvent::EndFile { ref reason } if reason == "error" => {
            state.notify("Playback failed; skipping track", true);
            super::queue::next_track(&mut state.domain)
        }
        PlaybackEvent::Shutdown => {
            state.domain.mpv_ready = false;
            if status_before != state.domain.playback.status {
                state.notify("mpv disconnected", true);
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

impl DomainState {
    /// Mirror a playback event into the snapshot (subset of controller logic).
    fn playback_status_from(&mut self, event: &PlaybackEvent) {
        use crate::playback::PlaybackStatus;
        match event {
            PlaybackEvent::Started => {
                self.playback.status = PlaybackStatus::Playing;
            }
            PlaybackEvent::FileLoaded => self.mark_file_loaded(),
            PlaybackEvent::PositionChanged(position) => self.record_position(*position),
            PlaybackEvent::DurationChanged(duration) => self.record_duration(*duration),
            PlaybackEvent::PauseChanged(paused) => {
                self.playback.status = if *paused {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Playing
                };
            }
            PlaybackEvent::VolumeChanged(v) => {
                self.playback.volume = (*v).clamp(0.0, 100.0) as u8;
            }
            PlaybackEvent::MuteChanged(m) => self.playback.muted = *m,
            PlaybackEvent::SpeedChanged(s) => self.playback.speed = *s,
            PlaybackEvent::EndFile { .. } => self.playback.status = PlaybackStatus::Stopped,
            PlaybackEvent::PlaybackError(_) | PlaybackEvent::Shutdown => {
                self.playback.status = PlaybackStatus::Idle;
            }
            PlaybackEvent::Connected => self.mpv_ready = true,
        }
    }
}
