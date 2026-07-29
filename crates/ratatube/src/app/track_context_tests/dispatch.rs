use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use crate::app::action::{
    Action, ExternalCommandKind, ExternalCommandTarget, NavigationAction, PlaybackAction,
    QueueAction,
};
use crate::app::state::{TrackContextMenuState, View};
use crate::app::tests::test_app;
use crate::app::track_context::{TrackContextAction, resolve_track_context};
use crate::media::search::SearchState;
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::track;

pub(super) fn open_menu_for_action(app: &mut crate::app::App, action: TrackContextAction) {
    let context = resolve_track_context(&app.state, app.history.as_deref()).expect("context");
    let selected = context
        .actions
        .iter()
        .position(|candidate| candidate == &action)
        .expect("action available");
    app.state.ui.track_context_menu = Some(TrackContextMenuState { context, selected });
}

async fn submit(app: &mut crate::app::App) -> (mpsc::Sender<Action>, mpsc::Receiver<Action>) {
    let (action_tx, action_rx) = mpsc::channel(2);
    app.handle_action(
        Action::Navigation(NavigationAction::SubmitTrackContext),
        &action_tx,
    )
    .await;
    (action_tx, action_rx)
}

async fn receive(action_rx: &mut mpsc::Receiver<Action>) -> Action {
    timeout(Duration::from_millis(100), action_rx.recv())
        .await
        .expect("dispatch timed out")
        .expect("dispatch channel closed")
}

#[tokio::test]
async fn track_context_menu_dispatches_typed_actions_with_the_resolved_track() {
    for context_action in [
        TrackContextAction::PlayNow,
        TrackContextAction::PlayNext,
        TrackContextAction::AddToQueue,
        TrackContextAction::VisitChannel,
    ] {
        let (_temp, mut app) = test_app();
        let selected = track("typed", "Typed track");
        app.state.ui.view = View::Search;
        app.state.domain.search = SearchState::Results {
            query: "typed".to_string(),
            tracks: vec![selected],
        };
        open_menu_for_action(&mut app, context_action.clone());

        let (_, mut action_rx) = submit(&mut app).await;
        let dispatched = receive(&mut action_rx).await;

        match (context_action, dispatched) {
            (TrackContextAction::PlayNow, Action::Playback(PlaybackAction::PlayTrack(track))) => {
                assert_eq!(track.id, "typed");
            }
            (TrackContextAction::PlayNext, Action::Queue(QueueAction::AddNext(track))) => {
                assert_eq!(track.id, "typed");
            }
            (TrackContextAction::AddToQueue, Action::Queue(QueueAction::AddToQueue(track))) => {
                assert_eq!(track.id, "typed");
            }
            (
                TrackContextAction::VisitChannel,
                Action::Navigation(NavigationAction::VisitChannel(track)),
            ) => assert_eq!(track.id, "typed"),
            (expected, actual) => panic!("{expected:?} dispatched unexpected action: {actual:?}"),
        }
        assert!(app.state.ui.track_context_menu.is_none());
    }
}

#[tokio::test]
async fn track_context_menu_dispatches_exact_modal_transition_per_action() {
    for context_action in [
        TrackContextAction::AddToPlaylist,
        TrackContextAction::ShowDetails,
    ] {
        let (_temp, mut app) = test_app();
        let selected = track("typed", "Typed track");
        app.state.ui.view = View::Search;
        app.state.domain.search = SearchState::Results {
            query: "typed".to_string(),
            tracks: vec![selected],
        };
        open_menu_for_action(&mut app, context_action.clone());

        let (_, mut action_rx) = submit(&mut app).await;

        assert!(action_rx.try_recv().is_err());
        match context_action {
            TrackContextAction::AddToPlaylist => {
                assert_eq!(
                    app.state
                        .ui
                        .picker
                        .as_ref()
                        .map(|picker| picker.track.id.as_str()),
                    Some("typed")
                );
                assert!(app.state.ui.track_details_modal.is_none());
            }
            TrackContextAction::ShowDetails => {
                assert_eq!(
                    app.state
                        .ui
                        .track_details_modal
                        .as_ref()
                        .map(|modal| modal.track.id.as_str()),
                    Some("typed")
                );
                assert!(app.state.ui.picker.is_none());
            }
            _ => unreachable!(),
        }
        assert!(app.state.ui.track_context_menu.is_none());
    }
}

#[tokio::test]
async fn track_context_menu_keeps_browser_and_clipboard_failures_open() {
    for (action, expected_command) in [
        (
            TrackContextAction::OpenInBrowser,
            ExternalCommandKind::Browser,
        ),
        (TrackContextAction::CopyUrl, ExternalCommandKind::Clipboard),
    ] {
        let (_temp, mut app) = test_app();
        let mut selected = track("unsafe", "Unsafe URL");
        selected.webpage_url = "file:///tmp/not-a-video".to_string();
        app.state.ui.view = View::Search;
        app.state.domain.search = SearchState::Results {
            query: "unsafe".to_string(),
            tracks: vec![selected],
        };
        open_menu_for_action(&mut app, action);

        let (action_tx, mut action_rx) = submit(&mut app).await;

        assert!(app.state.ui.track_context_menu.is_some());
        assert!(app.state.ui.notification.is_none());
        let completion = receive(&mut action_rx).await;
        assert!(matches!(
            completion,
            Action::Navigation(NavigationAction::ExternalCommandCompleted {
                command,
                target: ExternalCommandTarget::TrackContext { ref track_id, .. },
                result: Err(_),
                ..
            }) if command == expected_command && track_id == "unsafe"
        ));
        app.handle_action(completion, &action_tx).await;
        assert!(
            app.state
                .ui
                .notification
                .as_ref()
                .is_some_and(|notice| notice.is_error)
        );
    }
}

#[tokio::test]
async fn track_context_menu_queue_removal_revalidates_exact_occurrence_before_mutation() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Queue;
    app.state
        .domain
        .queue
        .push(track("duplicate", "First occurrence"));
    app.state
        .domain
        .queue
        .push(track("duplicate", "Second occurrence"));
    open_menu_for_action(
        &mut app,
        TrackContextAction::RemoveFromQueue { order_index: 0 },
    );
    let (action_tx, mut action_rx) = submit(&mut app).await;
    let removal = receive(&mut action_rx).await;
    app.state.domain.queue.order.swap(0, 1);

    app.handle_action(removal, &action_tx).await;

    assert_eq!(app.state.domain.queue.tracks.len(), 2);
    assert_eq!(
        app.state
            .ui
            .notification
            .as_ref()
            .map(|notice| notice.message.as_str()),
        Some("Queue changed; removal cancelled")
    );
}

#[tokio::test]
async fn track_context_menu_playlist_removal_revalidates_exact_occurrence_before_mutation() {
    let (_temp, mut app) = test_app();
    let mut playlist = Playlist::new("Duplicates");
    let playlist_id = playlist.id.clone();
    playlist
        .tracks
        .push(PlaylistTrack::from(&track("duplicate", "First occurrence")));
    playlist.tracks.push(PlaylistTrack::from(&track(
        "duplicate",
        "Second occurrence",
    )));
    app.state.ui.view = View::PlaylistDetail;
    app.state.domain.playlists.push(playlist);
    app.state.ui.selected_playlist = Some(0);
    open_menu_for_action(
        &mut app,
        TrackContextAction::RemoveFromPlaylist {
            playlist_id,
            track_index: 0,
        },
    );
    let (action_tx, mut action_rx) = submit(&mut app).await;
    let removal = receive(&mut action_rx).await;
    app.state.domain.playlists[0].tracks.swap(0, 1);

    app.handle_action(removal, &action_tx).await;

    assert_eq!(app.state.domain.playlists[0].tracks.len(), 2);
    assert_eq!(
        app.state
            .ui
            .notification
            .as_ref()
            .map(|notice| notice.message.as_str()),
        Some("Playlist changed; removal cancelled")
    );
}
