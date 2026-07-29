use super::*;

fn track(id: &str) -> Track {
    Track::new(id, id, "artist")
}

fn queue_with(ids: &[&str]) -> Queue {
    let mut queue = Queue::default();
    for id in ids {
        queue.push(track(id));
    }
    queue
}

/// Track ids in play order.
fn order_ids(queue: &Queue) -> Vec<&str> {
    queue
        .order
        .iter()
        .map(|&index| queue.tracks[index].id.as_str())
        .collect()
}

/// Track ids the stored play history points at, oldest first.
fn history_ids(queue: &Queue) -> Vec<&str> {
    queue
        .play_history
        .iter()
        .map(|&position| queue.tracks[queue.order[position]].id.as_str())
        .collect()
}

#[test]
fn effective_next_follows_linear_order_without_mutation() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.position = Some(0);
    queue.play_history = vec![2];
    let before = queue.clone();
    assert_eq!(
        queue.effective_next().map(|track| track.id.as_str()),
        Some("b")
    );
    assert_eq!(queue.position, before.position);
    assert_eq!(queue.order, before.order);
    assert_eq!(queue.play_history, before.play_history);
}

#[test]
fn effective_next_obeys_terminal_and_repeat_modes() {
    let mut queue = queue_with(&["a", "b"]);
    queue.position = Some(1);
    assert!(queue.effective_next().is_none());
    queue.repeat = RepeatMode::Queue;
    assert_eq!(
        queue.effective_next().map(|track| track.id.as_str()),
        Some("a")
    );
    queue.repeat = RepeatMode::Track;
    assert!(queue.effective_next().is_none());
}

#[test]
fn effective_next_uses_existing_shuffled_order() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.shuffle = true;
    queue.order = vec![2, 0, 1];
    queue.position = Some(0);
    assert_eq!(
        queue.effective_next().map(|track| track.id.as_str()),
        Some("a")
    );
    assert_eq!(queue.order, vec![2, 0, 1]);
}

#[test]
fn effective_next_rejects_invalid_position_without_panicking() {
    let mut queue = queue_with(&["a"]);
    queue.position = Some(usize::MAX);
    assert!(queue.effective_next().is_none());
}

#[test]
fn push_next_inserts_after_current() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.position = Some(0);
    queue.push_next(track("x"));
    let ids = queue
        .order
        .iter()
        .map(|&i| queue.tracks[i].id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["a", "x", "b", "c"]);
}

#[test]
fn advance_stops_at_end_when_repeat_off() {
    let mut queue = queue_with(&["a", "b"]);
    assert_eq!(queue.advance().map(|track| track.id.as_str()), Some("a"));
    assert_eq!(queue.advance().map(|track| track.id.as_str()), Some("b"));
    assert!(queue.advance().is_none());
    assert_eq!(queue.position, None);
}

#[test]
fn repeat_track_stays() {
    let mut queue = queue_with(&["a", "b"]);
    queue.repeat = RepeatMode::Track;
    queue.position = Some(0);
    assert_eq!(queue.advance().map(|track| track.id.as_str()), Some("a"));
}

#[test]
fn repeat_queue_wraps() {
    let mut queue = queue_with(&["a", "b"]);
    queue.repeat = RepeatMode::Queue;
    queue.position = Some(1);
    assert_eq!(queue.advance().map(|track| track.id.as_str()), Some("a"));
}

#[test]
fn previous_restarts_when_past_threshold() {
    let mut queue = queue_with(&["a", "b"]);
    queue.position = Some(1);
    assert_eq!(queue.previous(10, 5), PreviousOutcome::RestartCurrent);
}

#[test]
fn previous_uses_play_history_in_shuffle() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.set_shuffle(true);
    queue.position = Some(0);
    let first = queue.current().expect("current").id.clone();
    queue.advance();
    assert_ne!(first, queue.current().expect("current").id);
    assert_eq!(queue.previous(2, 5), PreviousOutcome::PlayPrevious);
    assert_eq!(queue.current().expect("current").id, first);
}

#[test]
fn reorder_keeps_current_selection() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.position = Some(2);
    queue.reorder(0, 1);
    assert_eq!(queue.current().expect("current").id, "c");
    assert_eq!(queue.position, Some(2));
}

#[test]
fn previous_still_reaches_the_prior_track_after_a_reorder() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.advance();
    queue.advance();
    assert_eq!(queue.current().expect("current").id, "b");
    assert_eq!(history_ids(&queue), vec!["a"]);
    queue.reorder(1, 0);
    assert_eq!(order_ids(&queue), vec!["b", "a", "c"]);
    assert_eq!(queue.current().expect("current").id, "b");
    assert_eq!(history_ids(&queue), vec!["a"]);
    assert_eq!(queue.previous(0, 5), PreviousOutcome::PlayPrevious);
    assert_eq!(queue.current().expect("current").id, "a");
    assert!(queue.validate().is_ok());
}

#[test]
fn reorder_forward_keeps_play_history_on_the_same_tracks() {
    let mut queue = queue_with(&["a", "b", "c", "d"]);
    queue.position = Some(3);
    queue.play_history = vec![0, 2];
    assert_eq!(history_ids(&queue), vec!["a", "c"]);
    queue.reorder(0, 2);
    assert_eq!(order_ids(&queue), vec!["b", "c", "a", "d"]);
    assert_eq!(queue.play_history, vec![2, 1]);
    assert_eq!(history_ids(&queue), vec!["a", "c"]);
    assert_eq!(queue.current().expect("current").id, "d");
    assert!(queue.validate().is_ok());
}

#[test]
fn reorder_backward_keeps_play_history_on_the_same_tracks() {
    let mut queue = queue_with(&["a", "b", "c", "d"]);
    queue.position = Some(0);
    queue.play_history = vec![1, 3];
    assert_eq!(history_ids(&queue), vec!["b", "d"]);
    queue.reorder(3, 1);
    assert_eq!(order_ids(&queue), vec!["a", "d", "b", "c"]);
    assert_eq!(queue.play_history, vec![2, 1]);
    assert_eq!(history_ids(&queue), vec!["b", "d"]);
    assert_eq!(queue.current().expect("current").id, "a");
    assert!(queue.validate().is_ok());
}

#[test]
fn push_next_keeps_play_history_on_the_same_tracks() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.position = Some(0);
    queue.play_history = vec![2];
    assert_eq!(history_ids(&queue), vec!["c"]);
    queue.push_next(track("x"));
    assert_eq!(order_ids(&queue), vec!["a", "x", "b", "c"]);
    assert_eq!(queue.play_history, vec![3]);
    assert_eq!(history_ids(&queue), vec!["c"]);
    assert_eq!(queue.position, Some(0));
    assert!(queue.validate().is_ok());
}

#[test]
fn unshuffle_restores_identity_order() {
    let mut queue = queue_with(&["a", "b", "c", "d"]);
    queue.position = Some(0);
    queue.set_shuffle(true);
    let current = queue.current().expect("current").id.clone();
    queue.set_shuffle(false);
    let ids = queue
        .order
        .iter()
        .map(|&i| queue.tracks[i].id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["a", "b", "c", "d"]);
    assert_eq!(queue.current().expect("current").id, current);
}

#[test]
fn validation_rejects_invalid_structures() {
    let mut queue = queue_with(&["a", "b"]);
    queue.order = vec![99, 0];
    assert!(queue.validate().is_err());
    queue.order = vec![0, 0];
    assert!(queue.validate().is_err());
    queue.order = vec![1];
    assert!(queue.validate().is_err());
    queue.order = vec![0, 1];
    queue.position = Some(2);
    assert!(queue.validate().is_err());
}

#[test]
fn validation_accepts_complete_shuffled_order() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.order = vec![2, 0, 1];
    queue.position = Some(1);
    assert!(queue.validate().is_ok());
}

#[test]
fn remove_deletes_track_and_preserves_valid_indices() {
    let mut queue = queue_with(&["a", "b", "c"]);
    queue.order = vec![2, 0, 1];
    queue.position = Some(1);
    queue.play_history = vec![0, 2];
    assert_eq!(queue.remove_at(1).expect("removed").id, "a");
    assert_eq!(
        queue
            .tracks
            .iter()
            .map(|track| track.id.as_str())
            .collect::<Vec<_>>(),
        vec!["b", "c"]
    );
    assert_eq!(queue.order, vec![1, 0]);
    assert_eq!(queue.position, None);
    assert_eq!(queue.play_history, vec![0, 1]);
    assert!(queue.validate().is_ok());
}
