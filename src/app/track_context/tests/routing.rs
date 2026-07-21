use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc;

use crate::app::action::{Action, NavigationAction, PlaybackAction};
use crate::app::channel::{ChannelNavigationSnapshot, ChannelState};
use crate::app::state::{Focus, TrackContextMenuState, TrackDetailsModalState, View};
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

fn mouse_at(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn context_app() -> (tempfile::TempDir, crate::app::App) {
    let (temp, mut app) = test_app();
    app.state.ui.view = View::Search;
    app.state.domain.search = SearchState::Results {
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
    app.state.ui.track_context_menu = Some(TrackContextMenuState { context, selected });
}

#[tokio::test]
async fn rapid_context_then_quit_is_captured_before_quit_can_queue() {
    let (_temp, mut app) = context_app();
    let (action_tx, mut action_rx) = mpsc::channel(4);

    app.handle_key(key(KeyCode::Char('c')), &action_tx).await;
    assert!(app.state.ui.track_context_menu.is_some());

    app.handle_key(key(KeyCode::Char('q')), &action_tx).await;

    assert!(
        action_rx.try_recv().is_err(),
        "no background action may leak"
    );
    assert!(app.state.ui.running);
    assert!(app.state.ui.track_context_menu.is_some());
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

        assert!(app.state.ui.track_context_menu.is_none());
        match action {
            TrackContextAction::AddToPlaylist => assert!(app.state.ui.picker.is_some()),
            TrackContextAction::ShowDetails => assert!(app.state.ui.track_details_modal.is_some()),
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

    app.state.ui.track_context_menu = Some(TrackContextMenuState {
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

    app.state.ui.track_context_menu = None;
    app.state.ui.track_details_modal = Some(TrackDetailsModalState {
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
async fn scrolled_channel_double_click_targets_exact_visible_row() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Channel;
    app.state.domain.channel = Some(ChannelState {
        name: "Channel".into(),
        url: "https://www.youtube.com/channel/UC1/videos".into(),
        tracks: (0..6)
            .map(|index| track(&format!("track-{index}"), &format!("Track {index}")))
            .collect(),
        next_page: 1,
        exhausted: true,
        loading: false,
        error: None,
        return_to: ChannelNavigationSnapshot {
            view: View::Search,
            focus: Focus::Content,
            selected_index: 0,
        },
        previous: None,
    });
    app.state.ui.list_hit_area = ratatui::layout::Rect::new(4, 5, 40, 3);
    *app.state.ui.table_state.offset_mut() = 2;
    let (action_tx, mut action_rx) = mpsc::channel(2);
    let click = mouse_at(
        MouseEventKind::Down(crossterm::event::MouseButton::Left),
        6,
        6,
    );

    app.handle_mouse(click, &action_tx).await;
    assert_eq!(app.state.ui.selected_index, 3);
    assert!(action_rx.try_recv().is_err());
    app.handle_mouse(click, &action_tx).await;

    assert!(matches!(
        action_rx.try_recv(),
        Ok(Action::Playback(PlaybackAction::PlaySelected))
    ));
    app.handle_action(Action::Playback(PlaybackAction::PlaySelected), &action_tx)
        .await;
    assert_eq!(app.state.domain.queue.tracks[0].id, "track-3");
}

#[tokio::test]
async fn paste_routes_only_to_the_topmost_prompt_modal() {
    let (_temp, mut app) = context_app();
    let (action_tx, mut action_rx) = mpsc::channel(2);
    app.state.ui.prompt = Some(crate::app::state::PromptState {
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
    app.state.ui.track_context_menu = Some(TrackContextMenuState {
        context,
        selected: 0,
    });
    app.handle_paste("blocked".to_string(), &action_tx).await;
    assert!(action_rx.try_recv().is_err());
}
