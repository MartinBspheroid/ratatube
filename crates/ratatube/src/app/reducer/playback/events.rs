//! Playback-event snapshot bookkeeping.

use crate::app::reducer::Effect;
use crate::app::state::AppState;
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
