use std::time::Instant;

use super::*;
use crate::playback::PlaybackStatus;

fn playing_state() -> AppState {
    let mut state = AppState::new();
    state.queue.push(track("current"));
    state.queue.push(track("next"));
    state.queue.position = Some(0);
    state.current_track = state.queue.current().cloned();
    state.playback.duration_seconds = Some(100.0);
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
