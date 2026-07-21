use std::time::Instant;

use super::*;
use crate::playback::PlaybackStatus;

fn resolve_current(state: &mut AppState, operation_id: crate::app::operations::OperationId) {
    let position = state.domain.queue.position.expect("queue position");
    let track_id = state
        .domain
        .queue
        .current()
        .expect("current track")
        .id
        .clone();
    reduce(
        state,
        Action::Playback(PlaybackAction::PlaybackResolveStarted {
            operation_id,
            queue_position: position,
            track_id: track_id.clone(),
        }),
    );
    reduce(
        state,
        Action::Playback(PlaybackAction::PlaybackResolved {
            operation_id,
            queue_position: position,
            track_id,
            url: "https://stream.invalid/track".to_string(),
        }),
    );
}

#[test]
fn playback_restart_after_resolution_cannot_unlock_stale_timing() {
    let mut state = AppState::new();
    state.domain.queue.push(track("current"));
    state.domain.queue.push(track("next"));
    state.domain.queue.position = Some(0);
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Playback);
    resolve_current(&mut state, ticket.id());

    reduce(
        &mut state,
        playback_event(PlaybackEvent::DurationChanged(100.0)),
    );
    reduce(
        &mut state,
        playback_event(PlaybackEvent::PositionChanged(90.0)),
    );
    reduce(&mut state, playback_event(PlaybackEvent::Started));
    assert_eq!(state.domain.track_transition.progress(Instant::now()), None);

    reduce(&mut state, playback_event(PlaybackEvent::FileLoaded));
    assert_eq!(state.domain.playback.duration_seconds, None);
    assert_eq!(state.domain.playback.position_seconds, 0.0);
}

#[test]
fn stale_prior_timing_is_cleared_by_a_new_resolution() {
    let mut state = AppState::new();
    state.domain.queue.push(track("current"));
    state.domain.queue.push(track("next"));
    state.domain.queue.position = Some(0);
    state.domain.current_track = state.domain.queue.current().cloned();
    state.begin_playback_occurrence();
    state.mark_file_loaded();
    state.record_duration(100.0);
    state.record_position(90.0);
    state.domain.playback.status = PlaybackStatus::Playing;
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Playback);
    resolve_current(&mut state, ticket.id());

    reduce(&mut state, playback_event(PlaybackEvent::FileLoaded));
    reduce(&mut state, playback_event(PlaybackEvent::Started));
    assert_eq!(state.domain.playback.duration_seconds, None);
    assert_eq!(state.domain.playback.position_seconds, 0.0);
    assert_eq!(state.domain.track_transition.progress(Instant::now()), None);
}

#[test]
fn accepted_session_resume_creates_an_occurrence_that_can_fire() {
    let mut state = AppState::new();
    let current = track("resume");
    state.domain.queue.push(current.clone());
    state.domain.queue.push(track("next"));
    state.domain.queue.position = Some(0);
    state.domain.current_track = Some(current.clone());
    state.domain.pending_resume = Some(crate::app::state::PendingResume {
        track: current,
        position_seconds: 85.0,
        armed: false,
        play_on_load: true,
    });

    reduce(
        &mut state,
        Action::Playback(PlaybackAction::SessionStreamResolved {
            operation_id: OperationRegistry::default()
                .start(OperationKind::Session)
                .id(),
            track_id: "resume".to_string(),
            url: "https://stream.invalid/resume".to_string(),
        }),
    );
    for event in [
        PlaybackEvent::FileLoaded,
        PlaybackEvent::DurationChanged(100.0),
        PlaybackEvent::PositionChanged(85.0),
        PlaybackEvent::Started,
    ] {
        reduce(&mut state, playback_event(event));
    }

    assert_ne!(state.domain.playback_occurrence, 0);
    assert!(
        state
            .domain
            .track_transition
            .progress(Instant::now())
            .is_some()
    );
}
