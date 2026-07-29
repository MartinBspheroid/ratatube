use tokio::sync::mpsc;

use super::track;
use crate::app::action::{Action, NavigationAction, PlaybackAction};
use crate::app::state::{AppState, TrackContextMenuState, View};
use crate::app::tests::test_app;
use crate::app::track_context::{TrackSource, open_track_context, resolve_track_context};
use crate::media::search::SearchState;
use crate::playback::PlaybackStatus;

#[test]
fn opening_without_a_track_notifies_exact_error_and_keeps_modal_absent() {
    let mut state = AppState::new();

    open_track_context(&mut state, None);

    assert!(state.ui.track_context_menu.is_none());
    let notification = state.ui.notification.expect("notification");
    assert_eq!(notification.message, "No track selected");
    assert!(notification.is_error);
}

#[tokio::test]
async fn nested_navigation_intents_open_move_submit_and_close_modal() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Search;
    app.state.domain.search = SearchState::Results {
        query: "intent".to_string(),
        tracks: vec![track("intent", "Intent track")],
    };
    let (action_tx, mut action_rx) = mpsc::channel(4);

    app.handle_action(
        Action::Navigation(NavigationAction::OpenTrackContext),
        &action_tx,
    )
    .await;
    assert_eq!(
        app.state
            .ui
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
    let menu = app.state.ui.track_context_menu.as_ref().expect("open menu");
    assert_eq!(menu.selected, menu.context.actions.len() - 1);

    app.handle_action(
        Action::Navigation(NavigationAction::MoveTrackContext(1)),
        &action_tx,
    )
    .await;
    app.handle_action(
        Action::Navigation(NavigationAction::SubmitTrackContext),
        &action_tx,
    )
    .await;
    assert!(app.state.ui.track_context_menu.is_none());
    assert!(matches!(
        action_rx.try_recv(),
        Ok(Action::Playback(PlaybackAction::PlayTrack(track))) if track.id == "intent"
    ));

    app.handle_action(
        Action::Navigation(NavigationAction::OpenTrackContext),
        &action_tx,
    )
    .await;
    app.handle_action(
        Action::Navigation(NavigationAction::CloseTrackContext),
        &action_tx,
    )
    .await;
    assert!(app.state.ui.track_context_menu.is_none());
}

#[tokio::test]
async fn context_menu_movement_handles_i32_boundaries_without_overflow() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Search;
    let selected = track("boundary", "Boundary track");
    app.state.domain.search = SearchState::Results {
        query: "boundary".to_string(),
        tracks: vec![selected.clone()],
    };
    app.state.domain.queue.push(selected);
    let context = resolve_track_context(&app.state, None).expect("context");
    assert_eq!(context.actions.len(), 7);
    app.state.ui.track_context_menu = Some(TrackContextMenuState {
        context,
        selected: usize::MAX,
    });
    let (action_tx, _action_rx) = mpsc::channel(4);

    app.handle_action(
        Action::Navigation(NavigationAction::MoveTrackContext(i32::MAX)),
        &action_tx,
    )
    .await;
    assert_eq!(
        app.state
            .ui
            .track_context_menu
            .as_ref()
            .map(|menu| menu.selected),
        Some(2)
    );

    app.handle_action(
        Action::Navigation(NavigationAction::MoveTrackContext(i32::MIN)),
        &action_tx,
    )
    .await;
    assert_eq!(
        app.state
            .ui
            .track_context_menu
            .as_ref()
            .map(|menu| menu.selected),
        Some(0)
    );
}

#[test]
fn modal_state_stores_resolved_context_and_selection() {
    let mut state = AppState::new();
    state.ui.view = View::Search;
    state.domain.search = SearchState::Results {
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

#[test]
fn selected_track_details_reuse_metadata_only_for_the_current_track() {
    let mut state = AppState::new();
    let current = track("current", "Current track");
    state.domain.current_track = Some(current.clone());
    state.domain.current_details = Some(crate::media::TrackDetails {
        view_count: Some(42),
        ..crate::media::TrackDetails::default()
    });
    state.domain.playback.status = PlaybackStatus::Playing;

    crate::app::reducer::reduce(
        &mut state,
        Action::Navigation(NavigationAction::ShowTrackDetails(track(
            "selected",
            "Selected track",
        ))),
    );

    let modal = state
        .ui
        .track_details_modal
        .as_ref()
        .expect("details modal");
    assert_eq!(modal.track.id, "selected");
    assert!(modal.details.is_none());
    assert_eq!(
        state
            .domain
            .current_details
            .as_ref()
            .and_then(|details| details.view_count),
        Some(42)
    );
    assert_eq!(state.domain.playback.status, PlaybackStatus::Playing);

    crate::app::reducer::reduce(
        &mut state,
        Action::Navigation(NavigationAction::ShowTrackDetails(current)),
    );
    assert_eq!(
        state
            .ui
            .track_details_modal
            .as_ref()
            .and_then(|modal| modal.details.as_ref())
            .and_then(|details| details.view_count),
        Some(42)
    );
}
