//! Playback queue selection and stream-resolution transitions.

use crate::app::action::PlaybackAction;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, OperationStatus, PlayingPane, View};
use crate::media::Track;
use crate::media::search::SearchState;
use crate::queue::PreviousOutcome;

/// Reduce playback queue selection and stream-resolution transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match action {
        PlaybackAction::PlayTrack(track) => play_track(&mut state.domain, track),
        PlaybackAction::PlayQueuePosition(position) => {
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
        // Resolved by the app layer through the existing pending-session flow.
        PlaybackAction::ResumeTrack { .. } => Vec::new(),
        PlaybackAction::SessionStreamResolved { track_id, .. } => {
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
        PlaybackAction::PlaySelected => {
            if matches!(state.ui.view, View::Queue)
                || (state.ui.view == View::NowPlaying
                    && state.ui.playing_pane == PlayingPane::Queue
                    && crate::ui::layout::Breakpoint::from_width(state.ui.screen_area.width)
                        == crate::ui::layout::Breakpoint::UltraWide)
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
        PlaybackAction::NextTrack => next_track(&mut state.domain),
        PlaybackAction::PreviousTrack => previous_track(&mut state.domain),
        PlaybackAction::PlaybackResolveStarted { operation_id, .. } => {
            state.domain.playback_resolution = OperationStatus::Loading { operation_id };
            Vec::new()
        }
        PlaybackAction::PlaybackResolved {
            operation_id,
            queue_position,
            track_id,
            ..
        } => {
            if resolved(&mut state.domain, operation_id, queue_position, &track_id) {
                // The domain switched tracks; drop the presentation caches.
                state.ui.thumbnail = None;
                state.ui.now_playing_scroll = 0;
            }
            Vec::new()
        }
        PlaybackAction::PlaybackResolveFailed {
            operation_id,
            message,
            ..
        } => {
            if resolve_failed(&mut state.domain, operation_id, &message) {
                state.notify(&format!("Playback unavailable: {message}"), true);
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Append `track` and start resolving it immediately.
fn play_track(domain: &mut DomainState, track: Track) -> Vec<Effect> {
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
fn previous_track(domain: &mut DomainState) -> Vec<Effect> {
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

/// Accept a resolution only for the active operation and matching occurrence;
/// true when the current track switched.
fn resolved(
    domain: &mut DomainState,
    operation_id: crate::app::operations::OperationId,
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
fn resolve_failed(
    domain: &mut DomainState,
    operation_id: crate::app::operations::OperationId,
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

/// Track selected in the active view, if the view lists tracks. The
/// selection index maps through the in-list filter when one is active.
fn selected_track(state: &AppState) -> Option<crate::media::Track> {
    let index = state.resolve_index(state.ui.selected_index);
    match state.ui.view {
        View::Search => match &state.domain.search {
            SearchState::Results { tracks, .. } => tracks.get(index).cloned(),
            _ => None,
        },
        View::Queue => state
            .domain
            .queue
            .order
            .get(index)
            .map(|&i| state.domain.queue.tracks[i].clone()),
        View::NowPlaying
            if state.ui.playing_pane == PlayingPane::Queue
                && crate::ui::layout::Breakpoint::from_width(state.ui.screen_area.width)
                    == crate::ui::layout::Breakpoint::UltraWide =>
        {
            state
                .domain
                .queue
                .order
                .get(index)
                .map(|&i| state.domain.queue.tracks[i].clone())
        }
        View::PlaylistDetail => state
            .ui
            .selected_playlist
            .and_then(|i| state.domain.playlists.get(i))
            .and_then(|p| p.tracks.get(index))
            .map(crate::media::Track::from),
        View::Channel => state
            .domain
            .channel
            .as_ref()
            .and_then(|channel| channel.tracks.get(index))
            .cloned(),
        _ => None,
    }
}
