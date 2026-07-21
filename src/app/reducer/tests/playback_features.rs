use super::*;

#[test]
fn chapter_jumps_seek_to_starts() {
    let mut state = AppState::new();
    state.domain.current_track = Some(track("mix"));
    state.domain.current_details = Some(crate::media::TrackDetails {
        chapters: vec![
            crate::media::Chapter {
                title: "a".into(),
                start_seconds: 0.0,
            },
            crate::media::Chapter {
                title: "b".into(),
                start_seconds: 100.0,
            },
            crate::media::Chapter {
                title: "c".into(),
                start_seconds: 200.0,
            },
        ],
        ..Default::default()
    });
    state.domain.playback.position_seconds = 120.0;
    assert_eq!(
        reduce(&mut state, Action::Playback(PlaybackAction::NextChapter)),
        vec![Effect::SeekTo(200.0)]
    );
    // More than 3s into a chapter: restart it first.
    assert_eq!(
        reduce(
            &mut state,
            Action::Playback(PlaybackAction::PreviousChapter)
        ),
        vec![Effect::SeekTo(100.0)]
    );
    // Near the chapter start: go to the previous one.
    state.domain.playback.position_seconds = 101.0;
    assert_eq!(
        reduce(
            &mut state,
            Action::Playback(PlaybackAction::PreviousChapter)
        ),
        vec![Effect::SeekTo(0.0)]
    );
    // No chapters: no effects.
    state.domain.current_details = None;
    assert!(reduce(&mut state, Action::Playback(PlaybackAction::NextChapter)).is_empty());
}

#[test]
fn speed_steps_clamp() {
    let mut state = AppState::new();
    assert_eq!(
        reduce(&mut state, Action::Playback(PlaybackAction::SpeedUp)),
        vec![Effect::SetSpeed(1.25)]
    );
    state.domain.playback.speed = 2.0;
    assert!(reduce(&mut state, Action::Playback(PlaybackAction::SpeedUp)).is_empty());
    state.domain.playback.speed = 0.5;
    assert!(reduce(&mut state, Action::Playback(PlaybackAction::SpeedDown)).is_empty());
    state.domain.playback.speed = 1.5;
    assert_eq!(
        reduce(&mut state, Action::Playback(PlaybackAction::SpeedReset)),
        vec![Effect::SetSpeed(1.0)]
    );
}

#[test]
fn sleep_timer_cycles_through_durations() {
    let mut state = AppState::new();
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::CycleSleepTimer),
    );
    assert_eq!(state.domain.sleep_timer.map(|t| t.minutes), Some(15));
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::CycleSleepTimer),
    );
    assert_eq!(state.domain.sleep_timer.map(|t| t.minutes), Some(30));
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::CycleSleepTimer),
    );
    assert_eq!(state.domain.sleep_timer.map(|t| t.minutes), Some(60));
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::CycleSleepTimer),
    );
    assert!(state.domain.sleep_timer.is_none());
}

#[test]
fn radio_tracks_dedup_and_restart_playback() {
    let mut state = AppState::new();
    state.domain.radio = true;
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Radio);
    state.domain.radio_operation = Some(ticket.id());
    state.domain.queue.push(track("known"));
    // Queue already exhausted: nothing playing, no position.
    state.domain.queue.position = None;
    let effects = reduce(
        &mut state,
        Action::Playback(PlaybackAction::RadioTracksLoaded {
            operation_id: ticket.id(),
            tracks: vec![track("known"), track("fresh1"), track("fresh2")],
        }),
    );
    assert_eq!(
        state.domain.queue.tracks.len(),
        3,
        "known track deduplicated"
    );
    assert_eq!(
        state.domain.queue.position,
        Some(1),
        "starts on first fresh track"
    );
    assert!(state.domain.current_track.is_none());
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
    );
}

#[test]
fn disabled_radio_discards_late_refill() {
    let mut state = AppState::new();
    state.domain.radio = true;
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Radio);
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::RadioRefillStarted {
            operation_id: ticket.id(),
        }),
    );
    reduce(&mut state, Action::Playback(PlaybackAction::ToggleRadio));
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::RadioTracksLoaded {
            operation_id: ticket.id(),
            tracks: vec![track("late")],
        }),
    );

    assert!(state.domain.queue.tracks.is_empty());
}

#[test]
fn mix_loaded_replaces_queue_and_enables_radio() {
    let mut state = AppState::new();
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Mix);
    state.domain.queue.push(track("old"));
    let effects = reduce(
        &mut state,
        Action::Playback(PlaybackAction::MixLoaded {
            operation_id: ticket.id(),
            title: "My Mix".to_string(),
            tracks: vec![track("m1"), track("m2")],
        }),
    );
    assert!(state.domain.radio);
    assert_eq!(state.domain.queue.tracks.len(), 2);
    assert_eq!(state.domain.queue.position, Some(0));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
    );
}

#[test]
fn eof_advances_queue() {
    let mut state = AppState::new();
    state.domain.queue.push(track("a"));
    state.domain.queue.push(track("b"));
    state.domain.queue.position = Some(0);
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(PlaybackEvent::EndFile {
            reason: "eof".to_string(),
        })),
    );
    assert_eq!(state.domain.queue.position, Some(1));
}
