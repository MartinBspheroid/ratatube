use super::*;

#[tokio::test]
async fn command_path_reduces_executes_and_persists_queue_action() {
    let (_temp, mut app) = test_app();
    let (action_tx, _action_rx) = mpsc::channel(4);
    app.handle_action(
        Action::Queue(QueueAction::AddToQueue(Track::new("id", "title", "artist"))),
        &action_tx,
    )
    .await;

    assert_eq!(app.state.queue.tracks.len(), 1);
    let restored = crate::queue::service::load(&app.paths.queue_file()).expect("saved queue");
    assert_eq!(restored.tracks[0].id, "id");
}

#[tokio::test]
async fn home_enter_dispatches_for_every_populated_iteration_four_section() {
    let (_temp, mut app) = test_app();
    app.state.view = View::Home;
    let resume = Track::new("resume", "Resume", "Artist");
    app.state.pending_resume = Some(crate::app::state::PendingResume {
        track: resume,
        position_seconds: 30.0,
        armed: true,
        play_on_load: false,
    });
    let (action_tx, mut action_rx) = mpsc::channel(8);
    app.state.home_section = crate::app::state::HomeSection::Resume;
    app.handle_action(Action::Playback(PlaybackAction::PlaySelected), &action_tx)
        .await;
    assert!(matches!(
        action_rx.try_recv(),
        Ok(Action::Playback(PlaybackAction::PlayPause))
    ));

    let mut history = HistoryService::load(&app.paths.history_file(), 500).expect("history");
    history.record(crate::history::model::HistoryEntry::from_track(
        &Track::new("recent", "Recent", "Artist"),
        None,
        PlaybackOutcome::Stopped,
        10,
    ));
    app.history = Some(history);
    app.state.home_section = crate::app::state::HomeSection::Recent;
    app.state.selected_index = 0;
    app.handle_action(Action::Playback(PlaybackAction::PlaySelected), &action_tx)
        .await;
    assert!(
        matches!(action_rx.try_recv(), Ok(Action::Playback(PlaybackAction::PlayTrack(track))) if track.id == "recent")
    );

    let playlist = Playlist::new("Home playlist");
    let playlist_id = playlist.id.clone();
    app.state.playlists.push(playlist);
    app.state.home_section = crate::app::state::HomeSection::Playlists;
    app.state.selected_index = 0;
    app.handle_action(Action::Playback(PlaybackAction::PlaySelected), &action_tx)
        .await;
    assert!(matches!(
        action_rx.try_recv(),
        Ok(Action::Playlists(PlaylistAction::LoadPlaylistIntoQueue(id))) if id == playlist_id
    ));
}

#[tokio::test]
async fn playlist_mutations_emit_persistable_activity() {
    use crate::history::activity::ActivityKind;

    let (_temp, mut app) = test_app();
    let playlist = Playlist::new("Target");
    app.playlists.save(&playlist).expect("save target");
    app.state.playlists.push(playlist);
    app.state.picker = Some(crate::app::state::PickerState {
        track: Track::new("track", "Added track", "Artist"),
        filter: String::new(),
        selected: 0,
    });
    let (action_tx, _action_rx) = mpsc::channel(4);
    app.handle_action(Action::Playlists(PlaylistAction::PickerSubmit), &action_tx)
        .await;
    assert_eq!(
        app.state.activity.entries().front().map(|event| event.kind),
        Some(ActivityKind::AddedToPlaylist)
    );

    app.state.import = Some(ImportState::Review {
        summary: crate::playlists::import::ImportSummary {
            remote_title: "Imported".to_string(),
            remote_url: "https://www.youtube.com/playlist?list=safe".to_string(),
            total_entries: 0,
            imported: 0,
            deleted: 0,
            private: 0,
            unavailable: 0,
            duplicates: 0,
            missing_id: 0,
            missing_title: 0,
        },
        playlist: Box::new(Playlist::new("Imported")),
    });
    app.handle_action(Action::Playlists(PlaylistAction::ConfirmImport), &action_tx)
        .await;
    assert_eq!(
        app.state.activity.entries().front().map(|event| event.kind),
        Some(ActivityKind::PlaylistImported)
    );
}
