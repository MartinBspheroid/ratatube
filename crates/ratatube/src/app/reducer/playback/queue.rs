//! Playback queue selection and stream-resolution transitions.
//!
//! Every entry point takes the payload it needs rather than a
//! `PlaybackAction`, so the family dispatcher in `super` is the only place
//! that enumerates the enum.

use crate::app::operations::OperationId;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, OperationStatus, PlayingPane, View};
use crate::media::Track;
use crate::media::search::SearchState;
use crate::queue::PreviousOutcome;

/// Append `track` and start resolving it immediately.
pub(super) fn play_track(domain: &mut DomainState, track: Track) -> Vec<Effect> {
    domain.queue.push(track);
    domain.bump_queue_revision();
    let pos = domain.queue.order.len() - 1;
    domain.queue.position = Some(pos);
    vec![
        Effect::ResolveAndPlay {
            track_index_in_queue: pos,
        },
        Effect::PersistQueue,
    ]
}

/// Jump the queue cursor to an order position and play it.
pub(super) fn play_queue_position(state: &mut AppState, position: usize) -> Vec<Effect> {
    if position < state.domain.queue.order.len() {
        state.domain.queue.position = Some(position);
        return vec![
            Effect::ResolveAndPlay {
                track_index_in_queue: position,
            },
            Effect::PersistQueue,
        ];
    }
    Vec::new()
}

/// Start the resumed session's playback occurrence once its stream arrives.
pub(super) fn session_stream_resolved(state: &mut AppState, track_id: &str) -> Vec<Effect> {
    if state
        .domain
        .pending_resume
        .as_ref()
        .is_some_and(|pending| pending.track.id == track_id)
    {
        state.domain.begin_playback_occurrence();
    }
    Vec::new()
}

/// Play whatever the active view has selected: a queue position in the queue
/// views, otherwise the selected track appended to the queue.
pub(super) fn play_selected(state: &mut AppState) -> Vec<Effect> {
    if matches!(state.ui.view, View::Queue)
        || (state.ui.view == View::NowPlaying && queue_pane_focused(state))
    {
        let position = state.resolve_index(state.ui.selected_index);
        if position < state.domain.queue.order.len() {
            state.domain.queue.position = Some(position);
            return vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: position,
                },
                Effect::PersistQueue,
            ];
        }
        return Vec::new();
    }
    if let Some(track) = selected_track(state) {
        return play_track(&mut state.domain, track);
    }
    Vec::new()
}

/// Advance the queue cursor and resolve the next track, if any.
pub(super) fn next_track(domain: &mut DomainState) -> Vec<Effect> {
    if domain.queue.advance().is_some() {
        let pos = domain.queue.position.unwrap_or(0);
        return vec![
            Effect::ResolveAndPlay {
                track_index_in_queue: pos,
            },
            Effect::PersistQueue,
        ];
    }
    Vec::new()
}

/// Restart the current track or step back, mirroring player conventions.
pub(super) fn previous_track(domain: &mut DomainState) -> Vec<Effect> {
    let position = domain.playback.position_seconds as u64;
    match domain.queue.previous(position, 5) {
        PreviousOutcome::RestartCurrent => vec![Effect::SeekTo(0.0)],
        PreviousOutcome::PlayPrevious => {
            let pos = domain.queue.position.unwrap_or(0);
            vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: pos,
                },
                Effect::PersistQueue,
            ]
        }
    }
}

/// Mark a stream resolution as the active one.
pub(super) fn resolve_started(state: &mut AppState, operation_id: OperationId) -> Vec<Effect> {
    state.domain.playback_resolution = OperationStatus::Loading { operation_id };
    Vec::new()
}

/// Accept a resolved stream, dropping the presentation caches when the current
/// track actually switched.
pub(super) fn resolved(
    state: &mut AppState,
    operation_id: OperationId,
    queue_position: usize,
    track_id: &str,
) -> Vec<Effect> {
    if accept_resolution(&mut state.domain, operation_id, queue_position, track_id) {
        // The domain switched tracks; drop the presentation caches.
        state.ui.thumbnail = None;
        state.ui.now_playing_scroll = 0;
    }
    Vec::new()
}

/// Record a failed resolution and warn, but only for the active operation.
pub(super) fn resolve_failed(
    state: &mut AppState,
    operation_id: OperationId,
    message: &str,
) -> Vec<Effect> {
    if record_resolve_failure(&mut state.domain, operation_id, message) {
        state.notify(&format!("Playback unavailable: {message}"), true);
    }
    Vec::new()
}

/// Accept a resolution only for the active operation and matching occurrence;
/// true when the current track switched.
fn accept_resolution(
    domain: &mut DomainState,
    operation_id: OperationId,
    queue_position: usize,
    track_id: &str,
) -> bool {
    if !matches!(
        domain.playback_resolution,
        OperationStatus::Loading { operation_id: active } if active == operation_id
    ) {
        return false;
    }
    let track = domain
        .queue
        .order
        .get(queue_position)
        .and_then(|index| domain.queue.tracks.get(*index))
        .filter(|track| track.id == track_id)
        .cloned();
    if let Some(track) = track {
        domain.begin_playback_occurrence();
        domain.current_track = Some(track);
        domain.current_details = None;
        domain.playback_resolution = OperationStatus::Idle;
        true
    } else {
        false
    }
}

/// Record a failed resolution only for the active operation.
fn record_resolve_failure(
    domain: &mut DomainState,
    operation_id: OperationId,
    message: &str,
) -> bool {
    if !matches!(
        domain.playback_resolution,
        OperationStatus::Loading { operation_id: active } if active == operation_id
    ) {
        return false;
    }
    domain.playback_resolution = OperationStatus::Failed {
        message: message.to_string(),
    };
    true
}

/// True when the ultra-wide Playing view has its queue pane focused, which is
/// the only case where that view's selection means a queue position.
fn queue_pane_focused(state: &AppState) -> bool {
    state.ui.playing_pane == PlayingPane::Queue
        && crate::ui::layout::Breakpoint::from_width(state.ui.screen_area.width)
            == crate::ui::layout::Breakpoint::UltraWide
}

/// Track selected in the active view, if the view lists tracks. The
/// selection index maps through the in-list filter when one is active.
fn selected_track(state: &AppState) -> Option<Track> {
    let index = state.resolve_index(state.ui.selected_index);
    match state.ui.view {
        View::Search => match &state.domain.search {
            SearchState::Results { tracks, .. } => tracks.get(index).cloned(),
            // Nothing is listed until results land.
            SearchState::Idle | SearchState::Searching { .. } | SearchState::Failed { .. } => None,
        },
        View::Queue => queue_track_at(state, index),
        View::NowPlaying => {
            if queue_pane_focused(state) {
                queue_track_at(state, index)
            } else {
                None
            }
        }
        View::PlaylistDetail => state
            .ui
            .selected_playlist
            .and_then(|i| state.domain.playlists.get(i))
            .and_then(|p| p.tracks.get(index))
            .map(Track::from),
        View::Channel => state
            .domain
            .channel
            .as_ref()
            .and_then(|channel| channel.tracks.get(index))
            .cloned(),
        // Home cards, the playlist catalog, history rows, and help are not
        // track lists this action can play from; the service layer resolves
        // the Home and History selections instead.
        View::Home | View::Playlists | View::History | View::Help => None,
    }
}

/// Track at play-order position `index`, if the queue has one there.
fn queue_track_at(state: &AppState, index: usize) -> Option<Track> {
    state
        .domain
        .queue
        .order
        .get(index)
        .map(|&i| state.domain.queue.tracks[i].clone())
}
