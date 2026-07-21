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
            state.domain.queue.load_tracks(tracks);
            state.bump_queue_revision();
            state.domain.queue.position = Some(0);
            state.domain.current_details = None;
            state.ui.thumbnail = None;
            state.ui.now_playing_scroll = 0;
            state.domain.radio = true;
            state.notify(&format!("Playing mix: {title}"), false);
            return vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: 0,
                },
                Effect::PersistQueue,
            ];
        }
        Action::Playback(PlaybackAction::RadioRefillStarted { operation_id }) => {
            state.domain.radio_operation = Some(operation_id);
        }
        Action::Playback(PlaybackAction::RadioTracksLoaded {
            operation_id,
            tracks,
        }) => {
            if !state.domain.radio || state.domain.radio_operation != Some(operation_id) {
                return Vec::new();
            }
            state.domain.radio_operation = None;
            let known: std::collections::HashSet<String> = state
                .domain
                .queue
                .tracks
                .iter()
                .map(|t| t.id.clone())
                .collect();
            let fresh: Vec<_> = tracks
                .into_iter()
                .filter(|t| !known.contains(&t.id))
                .take(10)
                .collect();
            if fresh.is_empty() {
                return Vec::new();
            }
            let first_new = state.domain.queue.order.len();
            let count = fresh.len();
            for track in fresh {
                state.domain.queue.push(track);
            }
            state.bump_queue_revision();
            state.notify(&format!("Radio: added {count} tracks"), false);
            // If playback had already run dry, start on the new tracks.
            if state.domain.queue.position.is_none() || state.domain.current_track.is_none() {
                state.domain.queue.position = Some(first_new);
                state.domain.current_details = None;
                state.ui.thumbnail = None;
                state.ui.now_playing_scroll = 0;
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
