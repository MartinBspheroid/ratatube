use std::time::Instant;

use super::*;
use crate::playback::PlaybackStatus;

fn resolve_current(state: &mut AppState, operation_id: crate::app::operations::OperationId) {
    let position = state.queue.position.expect("queue position");
    let track_id = state.queue.current().expect("current track").id.clone();
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

fn playing_state() -> AppState {
    let mut state = AppState::new();
    state.queue.push(track("current"));
    state.queue.push(track("next"));
    state.queue.position = Some(0);
    state.current_track = state.queue.current().cloned();
    state.begin_playback_occurrence();
    state.mark_file_loaded();
    state.record_duration(100.0);
    state.playback.status = PlaybackStatus::Playing;
    state
}

#[test]
fn playback_event_enters_transition_window_with_effective_next() {
    let mut state = playing_state();

    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(
            PlaybackEvent::PositionChanged(85.0),
        )),
    );

    assert!(state.track_transition.progress(Instant::now()).is_some());
}

#[test]
fn queue_mutation_removes_transition_when_no_next_remains() {
    let mut state = playing_state();
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(
            PlaybackEvent::PositionChanged(85.0),
        )),
    );
    assert!(state.track_transition.progress(Instant::now()).is_some());

    reduce(&mut state, Action::Queue(QueueAction::ClearQueueConfirmed));

    assert_eq!(state.track_transition.progress(Instant::now()), None);
}

#[test]
fn unknown_duration_never_starts_transition() {
    let mut state = playing_state();
    state.playback.duration_seconds = None;

    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(
            PlaybackEvent::PositionChanged(85.0),
        )),
    );

    assert_eq!(state.track_transition.progress(Instant::now()), None);
}

#[test]
fn new_resolution_rejects_previous_timing_until_fresh_events_arrive() {
    let mut state = playing_state();
    state.queue.repeat = crate::queue::RepeatMode::Queue;
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(
            PlaybackEvent::PositionChanged(85.0),
        )),
    );
    assert!(state.track_transition.progress(Instant::now()).is_some());

    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(PlaybackEvent::EndFile {
            reason: "eof".to_string(),
        })),
    );
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Playback);
    resolve_current(&mut state, ticket.id());
    reduce(&mut state, playback_event(PlaybackEvent::Started));

    assert_eq!(state.track_transition.progress(Instant::now()), None);

    reduce(&mut state, playback_event(PlaybackEvent::FileLoaded));
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(
            PlaybackEvent::DurationChanged(100.0),
        )),
    );
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(
            PlaybackEvent::PositionChanged(85.0),
        )),
    );
    assert!(state.track_transition.progress(Instant::now()).is_some());
}

#[test]
fn timing_before_file_loaded_is_discarded_and_never_claimed() {
    let mut state = AppState::new();
    state.queue.push(track("current"));
    state.queue.push(track("next"));
    state.queue.position = Some(0);
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Playback);
    resolve_current(&mut state, ticket.id());

    for event in [
        PlaybackEvent::DurationChanged(100.0),
        PlaybackEvent::PositionChanged(90.0),
    ] {
        reduce(
            &mut state,
            Action::Playback(PlaybackAction::PlaybackEvent(event)),
        );
    }

    assert_eq!(state.playback.duration_seconds, None);
    assert_eq!(state.playback.position_seconds, 0.0);
    assert_eq!(state.track_transition.progress(Instant::now()), None);

    reduce(&mut state, playback_event(PlaybackEvent::FileLoaded));
    reduce(&mut state, playback_event(PlaybackEvent::Started));
    assert_eq!(state.playback.duration_seconds, None);
    assert_eq!(state.playback.position_seconds, 0.0);
    assert_eq!(state.track_transition.progress(Instant::now()), None);

    reduce(
        &mut state,
        playback_event(PlaybackEvent::DurationChanged(100.0)),
    );
    reduce(
        &mut state,
        playback_event(PlaybackEvent::PositionChanged(90.0)),
    );
    assert!(state.track_transition.progress(Instant::now()).is_some());
}

#[test]
fn consecutive_same_track_occurrences_each_fire_once() {
    let mut state = AppState::new();
    state.queue.push(track("same"));
    state.queue.push(track("same"));
    state.queue.position = Some(0);
    state.queue.repeat = crate::queue::RepeatMode::Queue;
    let mut operations = OperationRegistry::default();

    for expected_position in 0..2 {
        state.queue.position = Some(expected_position);
        let ticket = operations.start(OperationKind::Playback);
        resolve_current(&mut state, ticket.id());
        assert_eq!(state.track_transition.progress(Instant::now()), None);
        for event in [
            PlaybackEvent::FileLoaded,
            PlaybackEvent::Started,
            PlaybackEvent::DurationChanged(100.0),
            PlaybackEvent::PositionChanged(85.0),
        ] {
            reduce(
                &mut state,
                Action::Playback(PlaybackAction::PlaybackEvent(event)),
            );
        }
        assert!(state.track_transition.progress(Instant::now()).is_some());
    }
}

#[test]
fn one_track_replay_gets_a_new_transition_occurrence() {
    let mut state = AppState::new();
    state.queue.push(track("same"));
    state.queue.position = Some(0);
    state.queue.repeat = crate::queue::RepeatMode::Queue;
    let mut operations = OperationRegistry::default();

    for _ in 0..2 {
        let ticket = operations.start(OperationKind::Playback);
        resolve_current(&mut state, ticket.id());
        assert_eq!(state.track_transition.progress(Instant::now()), None);
        for event in [
            PlaybackEvent::FileLoaded,
            PlaybackEvent::Started,
            PlaybackEvent::DurationChanged(100.0),
            PlaybackEvent::PositionChanged(85.0),
        ] {
            reduce(
                &mut state,
                Action::Playback(PlaybackAction::PlaybackEvent(event)),
            );
        }
        assert!(state.track_transition.progress(Instant::now()).is_some());
    }
}
