use super::*;
use crate::app::domain_event::{DomainEvent, DomainWatermark};
use crate::app::ui_sync::apply_domain_events;

fn events_for(state: &mut AppState, action: Action) -> Vec<DomainEvent> {
    let watermark = DomainWatermark::capture(&state.domain);
    let _ = reduce(state, action.clone());
    watermark.events_since(&state.domain, &action)
}

#[test]
fn queue_mutations_emit_queue_changed() {
    let mut state = AppState::new();
    let events = events_for(
        &mut state,
        Action::Queue(QueueAction::AddToQueue(track("a"))),
    );
    assert!(events.contains(&DomainEvent::QueueChanged), "{events:?}");
}

#[test]
fn search_submission_emits_search_changed() {
    let mut state = AppState::new();
    let events = events_for(
        &mut state,
        Action::Navigation(NavigationAction::SubmitSearch("query".into())),
    );
    assert!(events.contains(&DomainEvent::SearchChanged), "{events:?}");
}

#[test]
fn selection_movement_emits_no_domain_events() {
    let mut state = AppState::new();
    state.domain.queue.push(track("a"));
    state.domain.queue.push(track("b"));
    let events = events_for(&mut state, Action::Navigation(NavigationAction::SelectNext));
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn shuffle_toggle_emits_queue_changed() {
    let mut state = AppState::new();
    let events = events_for(&mut state, Action::Playback(PlaybackAction::ToggleShuffle));
    assert!(events.contains(&DomainEvent::QueueChanged), "{events:?}");
}

#[test]
fn apply_domain_events_clamps_selection_when_the_queue_shrinks() {
    let mut state = AppState::new();
    state.ui.view = View::Queue;
    state.domain.queue.push(track("a"));
    state.domain.queue.push(track("b"));
    state.ui.selected_index = 1;
    state.domain.queue.clear();
    apply_domain_events(&state.domain, &mut state.ui, &[DomainEvent::QueueChanged]);
    assert_eq!(state.ui.selected_index, 0);
}

#[test]
fn apply_domain_events_leaves_selection_alone_without_list_events() {
    let mut state = AppState::new();
    state.ui.view = View::Queue;
    state.domain.queue.push(track("a"));
    state.domain.queue.push(track("b"));
    state.ui.selected_index = 1;
    apply_domain_events(
        &state.domain,
        &mut state.ui,
        &[DomainEvent::PlaybackChanged],
    );
    assert_eq!(state.ui.selected_index, 1);
}
