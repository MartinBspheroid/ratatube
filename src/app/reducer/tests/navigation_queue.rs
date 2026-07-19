use super::*;

#[test]
fn superseded_search_results_are_discarded() {
    let mut state = AppState::new();
    reduce(
        &mut state,
        Action::Navigation(NavigationAction::SubmitSearch("first".to_string())),
    );
    reduce(
        &mut state,
        Action::Navigation(NavigationAction::SubmitSearch("second".to_string())),
    );
    let stale_generation = state.search_generation - 1;
    reduce(
        &mut state,
        Action::Navigation(NavigationAction::SearchCompleted {
            generation: stale_generation,
            tracks: vec![track("stale")],
        }),
    );
    assert!(matches!(state.search, SearchState::Searching { .. }));
}

#[test]
fn help_returns_to_the_view_that_opened_it_and_scrolls() {
    let mut state = AppState::new();
    state.view = View::Queue;
    reduce(&mut state, Action::Navigation(NavigationAction::OpenHelp));
    assert_eq!(state.view, View::Help);
    assert_eq!(state.help_return_view, View::Queue);
    reduce(
        &mut state,
        Action::Navigation(NavigationAction::ScrollHelp(7)),
    );
    assert_eq!(state.help_scroll, 7);
    reduce(&mut state, Action::Navigation(NavigationAction::CloseHelp));
    assert_eq!(state.view, View::Queue);
    assert_eq!(state.help_scroll, 0);
}

#[test]
fn queue_clear_requires_confirmation() {
    let mut state = AppState::new();
    state.queue.push(track("kept"));
    assert!(reduce(&mut state, Action::Queue(QueueAction::ClearQueue)).is_empty());
    assert_eq!(state.queue.tracks.len(), 1);
    assert!(state.confirm.is_some());
    let effects = reduce(&mut state, Action::Queue(QueueAction::ClearQueueConfirmed));
    assert!(state.queue.tracks.is_empty());
    assert!(effects.contains(&Effect::PersistQueue));
}

#[test]
fn removed_queue_item_can_be_undone() {
    let mut state = AppState::new();
    state.view = View::Queue;
    state.queue.push(track("a"));
    state.queue.push(track("b"));
    reduce(
        &mut state,
        Action::Queue(QueueAction::RemoveSelectedFromQueue),
    );
    assert_eq!(state.queue.tracks[0].id, "b");
    reduce(&mut state, Action::Queue(QueueAction::UndoQueueRemoval));
    let restored_ids: Vec<_> = state
        .queue
        .order
        .iter()
        .map(|&index| state.queue.tracks[index].id.as_str())
        .collect();
    assert_eq!(restored_ids, ["a", "b"]);
}

#[test]
fn notification_expiry_uses_elapsed_time_not_spinner_phase() {
    let now = std::time::Instant::now();
    let info = Notification::new_at("saved", false, now);
    let error = Notification::new_at("failed", true, now);
    assert!(!info.is_expired_at(now + std::time::Duration::from_secs(3)));
    assert!(info.is_expired_at(now + std::time::Duration::from_secs(5)));
    assert!(!error.is_expired_at(now + std::time::Duration::from_secs(5)));
    assert!(error.is_expired_at(now + std::time::Duration::from_secs(9)));
}

#[test]
fn exact_video_uses_metadata_fetch_effect_instead_of_search() {
    let mut state = AppState::new();
    let url = "https://www.youtube.com/watch?v=exact".to_string();
    let effects = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SubmitExactVideo(url.clone())),
    );

    assert!(matches!(
        effects.as_slice(),
        [Effect::RunExactVideo { url: effect_url, .. }] if effect_url == &url
    ));
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::RunSearch { .. }))
    );
}
