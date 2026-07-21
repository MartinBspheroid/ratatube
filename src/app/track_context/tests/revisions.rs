use tokio::sync::mpsc;

use crate::app::action::{Action, PlaybackAction, PlaylistAction};
use crate::app::reducer;
use crate::app::state::AppState;
use crate::app::tests::test_app;
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::track;

#[test]
fn queue_revision_advances_for_play_add_and_shuffle_order_changes() {
    let mut state = AppState::new();
    let initial = state.domain.queue_revision;

    reducer::reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlayTrack(track("one", "One"))),
    );
    assert_ne!(state.domain.queue_revision, initial);
    let after_add = state.domain.queue_revision;

    reducer::reduce(&mut state, Action::Playback(PlaybackAction::ToggleShuffle));
    assert_ne!(state.domain.queue_revision, after_add);
}

#[tokio::test]
async fn playlist_and_queue_load_mutations_advance_collection_revisions() {
    let (_temp, mut app) = test_app();
    let mut playlist = Playlist::new("Revision source");
    playlist
        .tracks
        .push(PlaylistTrack::from(&track("one", "One")));
    let playlist_id = playlist.id.clone();
    let playlist_revision = app.state.domain.playlists_revision;
    reducer::reduce(
        &mut app.state,
        Action::Playlists(PlaylistAction::PlaylistSaved(playlist)),
    );
    assert_ne!(app.state.domain.playlists_revision, playlist_revision);

    let queue_revision = app.state.domain.queue_revision;
    let (action_tx, _action_rx) = mpsc::channel(2);
    app.handle_action(
        Action::Playlists(PlaylistAction::LoadPlaylistIntoQueue(playlist_id)),
        &action_tx,
    )
    .await;
    assert_ne!(app.state.domain.queue_revision, queue_revision);
}
