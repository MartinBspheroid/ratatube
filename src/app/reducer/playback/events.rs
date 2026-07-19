//! Playback-event snapshot bookkeeping.

use crate::app::action::{Action, PlaybackAction};
use crate::app::reducer::{Effect, reduce};
use crate::app::state::AppState;
use crate::playback::PlaybackEvent;

/// Translate mpv events into follow-up actions (autoplay next on EOF, etc.).
pub(super) fn reduce_playback_event(state: &mut AppState, event: PlaybackEvent) -> Vec<Effect> {
    let status_before = state.playback.status;
    state.playback_status_from(&event);
    match event {
        PlaybackEvent::EndFile { ref reason } if reason == "eof" => {
            // Natural completion: advance the queue.
            reduce(state, Action::Playback(PlaybackAction::NextTrack))
        }
        PlaybackEvent::EndFile { ref reason } if reason == "error" => {
            state.notify("Playback failed; skipping track", true);
            reduce(state, Action::Playback(PlaybackAction::NextTrack))
        }
        PlaybackEvent::Shutdown => {
            state.mpv_ready = false;
            if status_before != state.playback.status {
                state.notify("mpv disconnected", true);
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

impl AppState {
    /// Mirror a playback event into the snapshot (subset of controller logic).
    fn playback_status_from(&mut self, event: &PlaybackEvent) {
        use crate::playback::PlaybackStatus;
        match event {
            PlaybackEvent::Started => self.playback.status = PlaybackStatus::Playing,
            PlaybackEvent::PositionChanged(p) => self.playback.position_seconds = *p,
            PlaybackEvent::DurationChanged(d) => self.playback.duration_seconds = Some(*d),
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
