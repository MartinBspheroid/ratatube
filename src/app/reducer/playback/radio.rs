//! Mix loading and radio-refill transitions.

use crate::app::action::{Action, PlaybackAction};
use crate::app::reducer::Effect;
use crate::app::state::AppState;

/// Reduce mix loading and radio-refill transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match Action::Playback(action) {
        Action::Playback(PlaybackAction::MixLoaded { title, tracks, .. }) => {
            if tracks.is_empty() {
                state.notify("Mix came back empty", true);
                return Vec::new();
            }
            state.queue.load_tracks(tracks);
            state.queue.position = Some(0);
            state.current_details = None;
            state.thumbnail = None;
            state.now_playing_scroll = 0;
            state.radio = true;
            state.notify(&format!("Playing mix: {title}"), false);
            return vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: 0,
                },
                Effect::PersistQueue,
            ];
        }
        Action::Playback(PlaybackAction::RadioRefillStarted { operation_id }) => {
            state.radio_operation = Some(operation_id);
        }
        Action::Playback(PlaybackAction::RadioTracksLoaded {
            operation_id,
            tracks,
        }) => {
            if !state.radio || state.radio_operation != Some(operation_id) {
                return Vec::new();
            }
            state.radio_operation = None;
            let known: std::collections::HashSet<String> =
                state.queue.tracks.iter().map(|t| t.id.clone()).collect();
            let fresh: Vec<_> = tracks
                .into_iter()
                .filter(|t| !known.contains(&t.id))
                .take(10)
                .collect();
            if fresh.is_empty() {
                return Vec::new();
            }
            let first_new = state.queue.order.len();
            let count = fresh.len();
            for track in fresh {
                state.queue.push(track);
            }
            state.notify(&format!("Radio: added {count} tracks"), false);
            // If playback had already run dry, start on the new tracks.
            if state.queue.position.is_none() || state.current_track.is_none() {
                state.queue.position = Some(first_new);
                state.current_details = None;
                state.thumbnail = None;
                state.now_playing_scroll = 0;
                return vec![
                    Effect::ResolveAndPlay {
                        track_index_in_queue: first_new,
                    },
                    Effect::PersistQueue,
                ];
            }
            return vec![Effect::PersistQueue];
        }

        // --- Add-to-playlist picker ----------------------------------------
        _ => {}
    }
    Vec::new()
}
