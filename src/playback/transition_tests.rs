use std::time::{Duration, Instant};

use super::{TRANSITION_DURATION, TrackTransitionState, TransitionInput};

fn input(
    occurrence: Option<u64>,
    remaining_seconds: Option<f64>,
    playing: bool,
    has_next: bool,
) -> TransitionInput {
    TransitionInput {
        occurrence,
        remaining_seconds,
        playing,
        has_next,
    }
}

#[test]
fn enters_final_window_once_for_current_track() {
    let now = Instant::now();
    let mut state = TrackTransitionState::default();
    state.update(input(Some(1), Some(16.0), true, true), now);
    assert_eq!(state.progress(now), None);

    state.update(input(Some(1), Some(15.0), true, true), now);
    assert_eq!(state.progress(now), Some(0.0));
    state.update(
        input(Some(1), Some(14.0), true, true),
        now + Duration::from_secs(1),
    );
    assert_eq!(
        state.progress(now + Duration::from_secs(1)),
        Some(1.0 / TRANSITION_DURATION.as_secs_f64())
    );
}

#[test]
fn pause_freezes_and_resume_continues_progress() {
    let now = Instant::now();
    let mut state = TrackTransitionState::default();
    state.update(input(Some(1), Some(15.0), true, true), now);
    state.update(
        input(Some(1), Some(14.0), false, true),
        now + Duration::from_secs(1),
    );
    let paused = state.progress(now + Duration::from_secs(20));
    assert_eq!(paused, Some(1.0 / TRANSITION_DURATION.as_secs_f64()));

    state.update(
        input(Some(1), Some(14.0), true, true),
        now + Duration::from_secs(20),
    );
    assert_eq!(
        state.progress(now + Duration::from_secs(21)),
        Some(2.0 / TRANSITION_DURATION.as_secs_f64())
    );
}

#[test]
fn seek_out_and_back_in_does_not_replay_for_same_track() {
    let now = Instant::now();
    let mut state = TrackTransitionState::default();
    state.update(input(Some(1), Some(15.0), true, true), now);
    state.update(
        input(Some(1), Some(30.0), true, true),
        now + TRANSITION_DURATION,
    );
    state.update(
        input(Some(1), Some(10.0), true, true),
        now + TRANSITION_DURATION + Duration::from_secs(1),
    );
    assert_eq!(
        state.progress(now + TRANSITION_DURATION + Duration::from_secs(1)),
        None
    );
}

#[test]
fn track_change_resets_and_unknown_or_missing_next_is_safe() {
    let now = Instant::now();
    let mut state = TrackTransitionState::default();
    state.update(input(Some(1), Some(10.0), true, true), now);
    state.update(
        input(Some(2), None, true, true),
        now + Duration::from_secs(1),
    );
    assert_eq!(state.progress(now + Duration::from_secs(1)), None);
    state.update(
        input(Some(2), Some(10.0), true, false),
        now + Duration::from_secs(2),
    );
    assert_eq!(state.progress(now + Duration::from_secs(2)), None);
    state.update(
        input(Some(2), Some(9.0), true, true),
        now + Duration::from_secs(3),
    );
    assert_eq!(state.progress(now + Duration::from_secs(3)), Some(0.0));
}

#[test]
fn removing_next_ends_transition_without_restarting_it() {
    let now = Instant::now();
    let mut state = TrackTransitionState::default();
    state.update(input(Some(1), Some(15.0), true, true), now);
    state.update(
        input(Some(1), Some(14.0), true, false),
        now + Duration::from_secs(1),
    );
    assert_eq!(state.progress(now + Duration::from_secs(1)), None);
    state.update(
        input(Some(1), Some(13.0), true, true),
        now + Duration::from_secs(2),
    );
    assert_eq!(state.progress(now + Duration::from_secs(2)), None);
}
