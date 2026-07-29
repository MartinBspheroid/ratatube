use super::*;

#[test]
fn seek_by_maps_to_a_relative_seek_effect() {
    let mut state = AppState::new();
    let effects = reduce(&mut state, Action::Playback(PlaybackAction::SeekBy(-42)));
    assert_eq!(effects, vec![Effect::SeekBy(-42)]);
}

#[test]
fn volume_by_maps_to_an_adjust_volume_effect() {
    let mut state = AppState::new();
    let effects = reduce(&mut state, Action::Playback(PlaybackAction::VolumeBy(5)));
    assert_eq!(effects, vec![Effect::AdjustVolume(5)]);
}

#[test]
fn move_track_reorders_explicit_positions_and_persists() {
    let mut state = AppState::new();
    state.domain.queue.push(track("a"));
    state.domain.queue.push(track("b"));
    state.domain.queue.push(track("c"));
    let revision_before = state.domain.queue_revision;
    let effects = reduce(
        &mut state,
        Action::Queue(QueueAction::MoveTrack { from: 0, to: 2 }),
    );
    assert_eq!(effects, vec![Effect::PersistQueue]);
    assert_ne!(state.domain.queue_revision, revision_before);
    let titles: Vec<&str> = state
        .domain
        .queue
        .order
        .iter()
        .map(|&index| state.domain.queue.tracks[index].title.as_str())
        .collect();
    assert_eq!(titles, vec!["b", "c", "a"]);
}

#[test]
fn move_track_out_of_bounds_is_a_no_op() {
    let mut state = AppState::new();
    state.domain.queue.push(track("a"));
    let effects = reduce(
        &mut state,
        Action::Queue(QueueAction::MoveTrack { from: 0, to: 5 }),
    );
    assert!(effects.is_empty());
}
