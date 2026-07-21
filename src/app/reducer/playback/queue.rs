//! Playback queue selection and stream-resolution transitions.

use crate::app::action::{Action, PlaybackAction};
use crate::app::reducer::{Effect, reduce as reduce_action};
use crate::app::state::{AppState, OperationStatus, PlayingPane, View};
use crate::media::search::SearchState;
use crate::queue::PreviousOutcome;

/// Reduce playback queue selection and stream-resolution transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match Action::Playback(action) {
        Action::Playback(PlaybackAction::PlayTrack(track)) => {
            state.domain.queue.push(track);
            state.bump_queue_revision();
            let pos = state.domain.queue.order.len() - 1;
            state.domain.queue.position = Some(pos);
            return vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: pos,
                },
                Effect::PersistQueue,
            ];
        }
        // Resolved by the app layer through the existing pending-session flow.
        Action::Playback(PlaybackAction::ResumeTrack { .. }) => {}
        Action::Playback(PlaybackAction::SessionStreamResolved { track_id, .. }) => {
            if state
                .domain
                .pending_resume
                .as_ref()
                .is_some_and(|pending| pending.track.id == track_id)
            {
                state.begin_playback_occurrence();
            }
        }
        Action::Playback(PlaybackAction::PlaySelected) => {
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
                return reduce_action(state, Action::Playback(PlaybackAction::PlayTrack(track)));
            }
        }
        Action::Playback(PlaybackAction::NextTrack) => {
            if state.domain.queue.advance().is_some() {
                let pos = state.domain.queue.position.unwrap_or(0);
                return vec![
                    Effect::ResolveAndPlay {
                        track_index_in_queue: pos,
                    },
                    Effect::PersistQueue,
                ];
            }
        }
        Action::Playback(PlaybackAction::PreviousTrack) => {
            let position = state.domain.playback.position_seconds as u64;
            match state.domain.queue.previous(position, 5) {
                PreviousOutcome::RestartCurrent => return vec![Effect::SeekTo(0.0)],
                PreviousOutcome::PlayPrevious => {
                    let pos = state.domain.queue.position.unwrap_or(0);
                    return vec![
                        Effect::ResolveAndPlay {
                            track_index_in_queue: pos,
                        },
                        Effect::PersistQueue,
                    ];
                }
            }
        }
        Action::Playback(PlaybackAction::PlaybackResolveStarted { operation_id, .. }) => {
            state.domain.playback_resolution = OperationStatus::Loading { operation_id };
        }
        Action::Playback(PlaybackAction::PlaybackResolved {
            operation_id,
            queue_position,
            track_id,
            ..
        }) => {
            if !matches!(
                state.domain.playback_resolution,
                OperationStatus::Loading { operation_id: active } if active == operation_id
            ) {
                return Vec::new();
            }
            let track = state
                .domain
                .queue
                .order
                .get(queue_position)
                .and_then(|index| state.domain.queue.tracks.get(*index))
                .filter(|track| track.id == track_id)
                .cloned();
            if let Some(track) = track {
                state.begin_playback_occurrence();
                state.domain.current_track = Some(track);
                state.domain.current_details = None;
                state.ui.thumbnail = None;
                state.ui.now_playing_scroll = 0;
                state.domain.playback_resolution = OperationStatus::Idle;
            }
        }
        Action::Playback(PlaybackAction::PlaybackResolveFailed {
            operation_id,
            message,
            ..
        }) => {
            if !matches!(
                state.domain.playback_resolution,
                OperationStatus::Loading { operation_id: active } if active == operation_id
            ) {
                return Vec::new();
            }
            state.domain.playback_resolution = OperationStatus::Failed {
                message: message.clone(),
            };
            state.notify(&format!("Playback unavailable: {message}"), true);
        }
        _ => {}
    }
    Vec::new()
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
