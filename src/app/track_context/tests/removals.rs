use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use crate::app::action::{Action, NavigationAction};
use crate::app::state::{TrackContextMenuState, View};
use crate::app::tests::test_app;
use crate::app::track_context::{TrackContextAction, resolve_track_context};
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::track;

async fn submit_removal(app: &mut crate::app::App, action: TrackContextAction) {
    let context = resolve_track_context(&app.state, None).expect("context");
    let selected = context
        .actions
        .iter()
        .position(|candidate| candidate == &action)
        .expect("removal action");
    app.state.track_context_menu = Some(TrackContextMenuState { context, selected });
    let (action_tx, mut action_rx) = mpsc::channel(2);
    app.handle_action(
        Action::Navigation(NavigationAction::SubmitTrackContext),
        &action_tx,
    )
    .await;
    let removal = timeout(Duration::from_millis(100), action_rx.recv())
        .await
        .expect("removal dispatch timed out")
        .expect("removal dispatch closed");
    app.handle_action(removal, &action_tx).await;
}

#[tokio::test]
async fn track_context_menu_removes_the_captured_queue_occurrence() {
    let (_temp, mut app) = test_app();
    app.state.view = View::Queue;
    app.state.queue.push(track("duplicate", "Keep first"));
    app.state.queue.push(track("duplicate", "Remove second"));
    app.state.selected_index = 1;

    submit_removal(
        &mut app,
        TrackContextAction::RemoveFromQueue { order_index: 1 },
    )
    .await;

    assert_eq!(app.state.queue.tracks.len(), 1);
    assert_eq!(app.state.queue.tracks[0].title, "Keep first");
}

#[tokio::test]
async fn track_context_menu_removes_the_captured_playlist_occurrence() {
    let (_temp, mut app) = test_app();
    let mut playlist = Playlist::new("Duplicates");
    let playlist_id = playlist.id.clone();
    playlist
        .tracks
        .push(PlaylistTrack::from(&track("duplicate", "Keep first")));
    playlist
        .tracks
        .push(PlaylistTrack::from(&track("duplicate", "Remove second")));
    app.state.view = View::PlaylistDetail;
    app.state.playlists.push(playlist);
    app.state.selected_playlist = Some(0);
    app.state.selected_index = 1;

    submit_removal(
        &mut app,
        TrackContextAction::RemoveFromPlaylist {
            playlist_id,
            track_index: 1,
        },
    )
    .await;

    assert_eq!(app.state.playlists[0].tracks.len(), 1);
    assert_eq!(app.state.playlists[0].tracks[0].title, "Keep first");
}
