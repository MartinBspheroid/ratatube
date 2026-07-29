use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use crate::app::action::{Action, NavigationAction, PlaylistAction, QueueAction};
use crate::app::state::{TrackContextMenuState, View};
use crate::app::tests::test_app;
use crate::app::track_context::{TrackContextAction, resolve_track_context};
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::track;

async fn captured_removal(app: &mut crate::app::App) -> Action {
    let context = resolve_track_context(&app.state, None).expect("context");
    let selected = context
        .actions
        .iter()
        .position(|action| {
            matches!(
                action,
                TrackContextAction::RemoveFromQueue { .. }
                    | TrackContextAction::RemoveFromPlaylist { .. }
            )
        })
        .expect("removal");
    app.state.ui.track_context_menu = Some(TrackContextMenuState { context, selected });
    let (action_tx, mut action_rx) = mpsc::channel(2);
    app.handle_action(
        Action::Navigation(NavigationAction::SubmitTrackContext),
        &action_tx,
    )
    .await;
    timeout(Duration::from_millis(100), action_rx.recv())
        .await
        .expect("removal timed out")
        .expect("removal channel closed")
}

#[tokio::test]
async fn reordered_exact_queue_clones_reject_captured_removal() {
    let (_temp, mut app) = test_app();
    let duplicate = track("same", "Exact clone");
    app.state.ui.view = View::Queue;
    app.state.domain.queue.push(duplicate.clone());
    app.state.domain.queue.push(duplicate);
    let removal = captured_removal(&mut app).await;
    let (action_tx, _action_rx) = mpsc::channel(2);

    app.handle_action(
        Action::Queue(QueueAction::MoveSelectedInQueue(1)),
        &action_tx,
    )
    .await;
    app.handle_action(removal, &action_tx).await;

    assert_eq!(app.state.domain.queue.tracks.len(), 2);
    assert_eq!(
        app.state
            .ui
            .notification
            .as_ref()
            .map(|notification| notification.message.as_str()),
        Some("Queue changed; removal cancelled")
    );
}

#[tokio::test]
async fn reordered_exact_playlist_clones_reject_removal_without_persisting_it() {
    let (_temp, mut app) = test_app();
    let mut playlist = Playlist::new("Exact clones");
    let stored = PlaylistTrack::from(&track("same", "Exact clone"));
    playlist.tracks.extend([stored.clone(), stored]);
    let playlist_id = playlist.id.clone();
    app.playlists
        .save(&playlist)
        .expect("save initial playlist");
    app.state.ui.view = View::PlaylistDetail;
    app.state.domain.playlists.push(playlist);
    app.state.ui.selected_playlist = Some(0);
    let removal = captured_removal(&mut app).await;
    let (action_tx, _action_rx) = mpsc::channel(2);

    app.handle_action(
        Action::Playlists(PlaylistAction::MoveSelectedInPlaylist(1)),
        &action_tx,
    )
    .await;
    app.handle_action(removal, &action_tx).await;

    assert_eq!(app.state.domain.playlists[0].tracks.len(), 2);
    assert_eq!(
        app.playlists
            .get(&playlist_id)
            .expect("persisted playlist")
            .tracks
            .len(),
        2
    );
    assert_eq!(
        app.state
            .ui
            .notification
            .as_ref()
            .map(|notification| notification.message.as_str()),
        Some("Playlist changed; removal cancelled")
    );
}
