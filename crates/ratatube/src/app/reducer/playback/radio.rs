//! Mix loading and radio-refill transitions.
//!
//! Every entry point takes the payload it needs rather than a
//! `PlaybackAction`, so the family dispatcher in `super` is the only place
//! that enumerates the enum.

use crate::app::operations::OperationId;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState};
use crate::media::Track;

/// Result of accepting a radio refill batch.
enum RadioOutcome {
    /// Stale operation, radio disabled, or nothing new to add.
    Ignored,
    /// Fresh tracks appended while playback continues.
    Appended { count: usize },
    /// Fresh tracks appended and playback started on them.
    Started { count: usize, first_new: usize },
}

/// Replace the queue with a fetched mix and play it from the top.
pub(super) fn mix_loaded(state: &mut AppState, title: String, tracks: Vec<Track>) -> Vec<Effect> {
    if tracks.is_empty() {
        state.notify("Mix came back empty", true);
        return Vec::new();
    }
    let effects = load_mix(&mut state.domain, tracks);
    // The domain switched tracks; drop the presentation caches.
    state.ui.thumbnail = None;
    state.ui.now_playing_scroll = 0;
    state.notify(&format!("Playing mix: {title}"), false);
    effects
}

/// Mark a radio refill as the active one, superseding any prior refill.
pub(super) fn refill_started(domain: &mut DomainState, operation_id: OperationId) -> Vec<Effect> {
    domain.radio_operation = Some(operation_id);
    Vec::new()
}

/// Accept a radio refill batch and announce what it added.
pub(super) fn tracks_loaded(
    state: &mut AppState,
    operation_id: OperationId,
    tracks: Vec<Track>,
) -> Vec<Effect> {
    match radio_tracks_loaded(&mut state.domain, operation_id, tracks) {
        RadioOutcome::Ignored => Vec::new(),
        RadioOutcome::Appended { count } => {
            state.notify(&format!("Radio: added {count} tracks"), false);
            vec![Effect::PersistQueue]
        }
        RadioOutcome::Started { count, first_new } => {
            state.notify(&format!("Radio: added {count} tracks"), false);
            state.ui.thumbnail = None;
            state.ui.now_playing_scroll = 0;
            vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: first_new,
                },
                Effect::PersistQueue,
            ]
        }
    }
}

/// Replace the queue with a loaded mix and start it from the top.
fn load_mix(domain: &mut DomainState, tracks: Vec<Track>) -> Vec<Effect> {
    domain.queue.load_tracks(tracks);
    domain.bump_queue_revision();
    domain.queue.position = Some(0);
    domain.current_details = None;
    domain.radio = true;
    vec![
        Effect::ResolveAndPlay {
            track_index_in_queue: 0,
        },
        Effect::PersistQueue,
    ]
}

/// Deduplicate and append a refill batch; start playback when it ran dry.
fn radio_tracks_loaded(
    domain: &mut DomainState,
    operation_id: OperationId,
    tracks: Vec<Track>,
) -> RadioOutcome {
    if !domain.radio || domain.radio_operation != Some(operation_id) {
        return RadioOutcome::Ignored;
    }
    domain.radio_operation = None;
    let known: std::collections::HashSet<String> =
        domain.queue.tracks.iter().map(|t| t.id.clone()).collect();
    let fresh: Vec<_> = tracks
        .into_iter()
        .filter(|t| !known.contains(&t.id))
        .take(10)
        .collect();
    if fresh.is_empty() {
        return RadioOutcome::Ignored;
    }
    let first_new = domain.queue.order.len();
    let count = fresh.len();
    for track in fresh {
        domain.queue.push(track);
    }
    domain.bump_queue_revision();
    // If playback had already run dry, start on the new tracks.
    if domain.queue.position.is_none() || domain.current_track.is_none() {
        domain.queue.position = Some(first_new);
        domain.current_details = None;
        RadioOutcome::Started { count, first_new }
    } else {
        RadioOutcome::Appended { count }
    }
}
