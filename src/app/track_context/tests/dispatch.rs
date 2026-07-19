use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use crate::app::action::{Action, NavigationAction, PlaybackAction, PlaylistAction, QueueAction};
use crate::app::state::{TrackContextMenuState, View};
use crate::app::tests::test_app;
use crate::app::track_context::{TrackContextAction, resolve_track_context};
use crate::media::search::SearchState;
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::track;

fn open_menu_for_action(app: &mut crate::app::App, action: TrackContextAction) {
    let context = resolve_track_context(&app.state, app.history.as_ref()).expect("context");
    let selected = context
        .actions
        .iter()
        .position(|candidate| candidate == &action)
        .expect("action available");
    app.state.track_context_menu = Some(TrackContextMenuState { context, selected });
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
        TrackContextAction::AddToPlaylist,
        TrackContextAction::VisitChannel,
        TrackContextAction::ShowDetails,
    ] {
        let (_temp, mut app) = test_app();
        let selected = track("typed", "Typed track");
        app.state.view = View::Search;
        app.state.search = SearchState::Results {
            query: "typed".to_string(),
            tracks: vec![selected],
        };
        open_menu_for_action(&mut app, context_action.clone());

        let (_, mut action_rx) = submit(&mut app).await;
        let dispatched = receive(&mut action_rx).await;

        let track_id = match dispatched {
            Action::Playback(PlaybackAction::PlayTrack(track))
            | Action::Queue(QueueAction::AddNext(track))
            | Action::Queue(QueueAction::AddToQueue(track))
            | Action::Playlists(PlaylistAction::OpenPlaylistPickerForTrack(track))
            | Action::Navigation(NavigationAction::VisitChannel(track))
            | Action::Navigation(NavigationAction::ShowTrackDetails(track)) => track.id,
            other => panic!("unexpected dispatch: {other:?}"),
        };
        assert_eq!(track_id, "typed");
        assert!(app.state.track_context_menu.is_none());
    }
}

#[tokio::test]
async fn track_context_menu_keeps_browser_and_clipboard_failures_open() {
    for action in [
        TrackContextAction::OpenInBrowser,
        TrackContextAction::CopyUrl,
    ] {
        let (_temp, mut app) = test_app();
        let mut selected = track("unsafe", "Unsafe URL");
        selected.webpage_url = "file:///tmp/not-a-video".to_string();
        app.state.view = View::Search;
        app.state.search = SearchState::Results {
            query: "unsafe".to_string(),
            tracks: vec![selected],
        };
        open_menu_for_action(&mut app, action);

        let (_, action_rx) = submit(&mut app).await;

        assert!(action_rx.is_empty());
        assert!(app.state.track_context_menu.is_some());
        assert!(
            app.state
                .notification
                .as_ref()
                .is_some_and(|notice| notice.is_error)
        );
    }
}

#[tokio::test]
async fn track_context_menu_queue_removal_revalidates_exact_occurrence_before_mutation() {
    let (_temp, mut app) = test_app();
    app.state.view = View::Queue;
    app.state.queue.push(track("duplicate", "First occurrence"));
    app.state
        .queue
        .push(track("duplicate", "Second occurrence"));
    open_menu_for_action(
        &mut app,
        TrackContextAction::RemoveFromQueue { order_index: 0 },
    );
    let (action_tx, mut action_rx) = submit(&mut app).await;
    let removal = receive(&mut action_rx).await;
    app.state.queue.order.swap(0, 1);

    app.handle_action(removal, &action_tx).await;

    assert_eq!(app.state.queue.tracks.len(), 2);
    assert_eq!(
        app.state
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
    app.state.view = View::PlaylistDetail;
    app.state.playlists.push(playlist);
    app.state.selected_playlist = Some(0);
    open_menu_for_action(
        &mut app,
        TrackContextAction::RemoveFromPlaylist {
            playlist_id,
            track_index: 0,
        },
    );
    let (action_tx, mut action_rx) = submit(&mut app).await;
    let removal = receive(&mut action_rx).await;
    app.state.playlists[0].tracks.swap(0, 1);

    app.handle_action(removal, &action_tx).await;

    assert_eq!(app.state.playlists[0].tracks.len(), 2);
    assert_eq!(
        app.state
            .notification
            .as_ref()
            .map(|notice| notice.message.as_str()),
        Some("Playlist changed; removal cancelled")
    );
}
