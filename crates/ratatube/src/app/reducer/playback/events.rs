//! Playback-event snapshot bookkeeping.

use crate::app::reducer::Effect;
use crate::app::state::AppState;
use crate::playback::PlaybackEvent;

/// Translate mpv events into follow-up effects (autoplay next on EOF, etc.).
pub(super) fn reduce_playback_event(state: &mut AppState, event: PlaybackEvent) -> Vec<Effect> {
    let status_before = state.domain.playback.status;
    state.domain.playback_status_from(&event);
    match event {
        PlaybackEvent::EndFile { ref reason } => end_file(state, reason),
        PlaybackEvent::Shutdown => {
            state.domain.mpv_ready = false;
            if status_before != state.domain.playback.status {
                state.notify("mpv disconnected", true);
            }
            Vec::new()
        }
        // Snapshot-only events: `playback_status_from` above already folded
        // each one into the playback snapshot, so no transition follows.
        PlaybackEvent::Connected
        | PlaybackEvent::Started
        | PlaybackEvent::FileLoaded
        | PlaybackEvent::PositionChanged(_)
        | PlaybackEvent::DurationChanged(_)
        | PlaybackEvent::PauseChanged(_)
        | PlaybackEvent::VolumeChanged(_)
        | PlaybackEvent::MuteChanged(_)
        | PlaybackEvent::SpeedChanged(_)
        | PlaybackEvent::AudioLevels(_)
        | PlaybackEvent::PlaybackError(_) => Vec::new(),
    }
}

/// React to a finished file by its stop reason: natural completion advances
/// the queue, an error advances with a warning, and any other reason (a
/// deliberate stop or a load that never started) stays put.
fn end_file(state: &mut AppState, reason: &str) -> Vec<Effect> {
    if reason == "eof" {
        return super::queue::next_track(&mut state.domain);
    }
    if reason == "error" {
        state.notify("Playback failed; skipping track", true);
        return super::queue::next_track(&mut state.domain);
    }
    Vec::new()
}
