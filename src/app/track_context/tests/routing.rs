use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc;

use crate::app::action::{Action, NavigationAction};
use crate::app::state::{TrackContextMenuState, TrackDetailsModalState, View};
use crate::app::tests::test_app;
use crate::app::track_context::{TrackContextAction, resolve_track_context};
use crate::media::search::SearchState;

use super::track;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn mouse(kind: MouseEventKind) -> MouseEvent {
    MouseEvent {
        kind,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    }
}

fn context_app() -> (tempfile::TempDir, crate::app::App) {
    let (temp, mut app) = test_app();
    app.state.view = View::Search;
    app.state.search = SearchState::Results {
        query: "context".to_string(),
        tracks: vec![track("context", "Context track")],
    };
    (temp, app)
}

fn select_action(app: &mut crate::app::App, action: TrackContextAction) {
    let context = resolve_track_context(&app.state, None).expect("context");
    let selected = context
        .actions
        .iter()
        .position(|candidate| candidate == &action)
        .expect("action");
    app.state.track_context_menu = Some(TrackContextMenuState { context, selected });
}

#[tokio::test]
async fn rapid_context_then_quit_is_captured_before_quit_can_queue() {
    let (_temp, mut app) = context_app();
    let (action_tx, mut action_rx) = mpsc::channel(4);

    app.handle_key(key(KeyCode::Char('c')), &action_tx).await;
    assert!(app.state.track_context_menu.is_some());

    app.handle_key(key(KeyCode::Char('q')), &action_tx).await;

    assert!(
        action_rx.try_recv().is_err(),
        "no background action may leak"
    );
    assert!(app.state.running);
    assert!(app.state.track_context_menu.is_some());
}

#[tokio::test]
async fn context_menu_replaces_itself_atomically_with_picker_or_details() {
    for action in [
        TrackContextAction::AddToPlaylist,
        TrackContextAction::ShowDetails,
    ] {
        let (_temp, mut app) = context_app();
        select_action(&mut app, action.clone());
        let (action_tx, mut action_rx) = mpsc::channel(2);

        app.handle_action(
            Action::Navigation(NavigationAction::SubmitTrackContext),
            &action_tx,
        )
        .await;

        assert!(app.state.track_context_menu.is_none());
        match action {
            TrackContextAction::AddToPlaylist => assert!(app.state.picker.is_some()),
            TrackContextAction::ShowDetails => assert!(app.state.track_details_modal.is_some()),
            _ => unreachable!(),
        }
        assert!(
            action_rx.try_recv().is_err(),
            "transition must not be deferred"
        );
    }
}

#[tokio::test]
async fn context_and_details_modals_block_all_background_mouse_actions() {
    let (_temp, mut app) = context_app();
    let context = resolve_track_context(&app.state, None).expect("context");
    let (action_tx, mut action_rx) = mpsc::channel(8);

    app.state.track_context_menu = Some(TrackContextMenuState {
        context,
        selected: 0,
    });
    app.handle_mouse(mouse(MouseEventKind::ScrollDown), &action_tx)
        .await;
    app.handle_mouse(
        mouse(MouseEventKind::Down(crossterm::event::MouseButton::Left)),
        &action_tx,
    )
    .await;
    assert!(action_rx.try_recv().is_err());

    app.state.track_context_menu = None;
    app.state.track_details_modal = Some(TrackDetailsModalState {
        track: track("details", "Details"),
        details: None,
    });
    app.handle_mouse(mouse(MouseEventKind::ScrollUp), &action_tx)
        .await;
    app.handle_mouse(
        mouse(MouseEventKind::Down(crossterm::event::MouseButton::Left)),
        &action_tx,
    )
    .await;
    assert!(action_rx.try_recv().is_err());
}

#[tokio::test]
async fn paste_routes_only_to_the_topmost_prompt_modal() {
    let (_temp, mut app) = context_app();
    let (action_tx, mut action_rx) = mpsc::channel(2);
    app.state.prompt = Some(crate::app::state::PromptState {
        purpose: crate::app::state::PromptPurpose::ImportPlaylistJson,
        buffer: String::new(),
    });

    app.handle_paste("prompt text".to_string(), &action_tx)
        .await;
    assert!(matches!(
        action_rx.try_recv(),
        Ok(Action::Playlists(
            crate::app::action::PlaylistAction::PromptPaste(text)
        )) if text == "prompt text"
    ));

    let context = resolve_track_context(&app.state, None).expect("context");
    app.state.track_context_menu = Some(TrackContextMenuState {
        context,
        selected: 0,
    });
    app.handle_paste("blocked".to_string(), &action_tx).await;
    assert!(action_rx.try_recv().is_err());
}
