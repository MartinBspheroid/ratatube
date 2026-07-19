use tokio::sync::mpsc;

use super::track;
use crate::app::action::{Action, NavigationAction};
use crate::app::state::{AppState, TrackContextMenuState, View};
use crate::app::tests::test_app;
use crate::app::track_context::{TrackSource, open_track_context, resolve_track_context};
use crate::media::search::SearchState;

#[test]
fn opening_without_a_track_notifies_exact_error_and_keeps_modal_absent() {
    let mut state = AppState::new();

    open_track_context(&mut state, None);

    assert!(state.track_context_menu.is_none());
    let notification = state.notification.expect("notification");
    assert_eq!(notification.message, "No track selected");
    assert!(notification.is_error);
}

#[tokio::test]
async fn nested_navigation_intents_open_move_submit_and_close_modal() {
    let (_temp, mut app) = test_app();
    app.state.view = View::Search;
    app.state.search = SearchState::Results {
        query: "intent".to_string(),
        tracks: vec![track("intent", "Intent track")],
    };
    let (action_tx, _action_rx) = mpsc::channel(4);

    app.handle_action(
        Action::Navigation(NavigationAction::OpenTrackContext),
        &action_tx,
    )
    .await;
    assert_eq!(
        app.state
            .track_context_menu
            .as_ref()
            .map(|menu| menu.selected),
        Some(0)
    );

    app.handle_action(
        Action::Navigation(NavigationAction::MoveTrackContext(-1)),
        &action_tx,
    )
    .await;
    let menu = app.state.track_context_menu.as_ref().expect("open menu");
    assert_eq!(menu.selected, menu.context.actions.len() - 1);

    app.handle_action(
        Action::Navigation(NavigationAction::SubmitTrackContext),
        &action_tx,
    )
    .await;
    assert!(app.state.track_context_menu.is_some());

    app.handle_action(
        Action::Navigation(NavigationAction::CloseTrackContext),
        &action_tx,
    )
    .await;
    assert!(app.state.track_context_menu.is_none());
}

#[test]
fn modal_state_stores_resolved_context_and_selection() {
    let mut state = AppState::new();
    state.view = View::Search;
    state.search = SearchState::Results {
        query: "modal".to_string(),
        tracks: vec![track("modal", "Modal track")],
    };
    let context = resolve_track_context(&state, None).expect("context");
    let menu = TrackContextMenuState {
        context,
        selected: 0,
    };

    assert_eq!(menu.selected, 0);
    assert_eq!(menu.context.source, TrackSource::Search);
}
