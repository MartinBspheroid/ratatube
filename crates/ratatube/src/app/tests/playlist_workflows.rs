use super::*;

#[tokio::test]
async fn by_id_wire_actions_reach_the_editing_handler_and_persist() {
    let (_temp, mut app) = test_app();
    let (action_tx, _action_rx) = mpsc::channel(8);
    app.handle_action(
        Action::Playlists(PlaylistAction::CreatePlaylist("Neon Pressure".into())),
        &action_tx,
    )
    .await;
    let id = app.state.domain.playlists[0].id.clone();

    // The rename, edit, add, and move commands arrive from clients as by-id
    // actions; each must actually mutate and persist, never be dropped.
    app.handle_action(
        Action::Playlists(PlaylistAction::RenamePlaylist {
            id: id.clone(),
            name: "Pressure - dnb/bass".into(),
        }),
        &action_tx,
    )
    .await;
    assert_eq!(app.state.domain.playlists[0].name, "Pressure - dnb/bass");

    app.handle_action(
        Action::Playlists(PlaylistAction::EditPlaylist {
            id: id.clone(),
            name: "Pressure".into(),
            description: "rolling basslines".into(),
        }),
        &action_tx,
    )
    .await;
    assert_eq!(app.state.domain.playlists[0].name, "Pressure");
    assert_eq!(
        app.state.domain.playlists[0].description,
        "rolling basslines"
    );

    for track_id in ["a", "b"] {
        app.handle_action(
            Action::Playlists(PlaylistAction::AddTrackToPlaylist {
                playlist_id: id.clone(),
                track: Track::new(track_id, track_id, "artist"),
            }),
            &action_tx,
        )
        .await;
    }
    assert_eq!(app.state.domain.playlists[0].tracks.len(), 2);

    app.handle_action(
        Action::Playlists(PlaylistAction::MoveTrackInPlaylist {
            id: id.clone(),
            from: 0,
            to: 1,
        }),
        &action_tx,
    )
    .await;
    assert_eq!(app.state.domain.playlists[0].tracks[0].id, "b");

    // Every mutation must be durable, not just in memory.
    let stored = app.playlists.list().expect("stored playlists");
    assert_eq!(stored[0].name, "Pressure");
    assert_eq!(stored[0].description, "rolling basslines");
    assert_eq!(stored[0].tracks.len(), 2);
    assert_eq!(stored[0].tracks[0].id, "b");
}

#[tokio::test]
async fn selection_based_rename_persists_instead_of_being_dropped() {
    let (_temp, mut app) = test_app();
    let (action_tx, _action_rx) = mpsc::channel(8);
    app.handle_action(
        Action::Playlists(PlaylistAction::CreatePlaylist("Draft".into())),
        &action_tx,
    )
    .await;
    app.state.ui.view = crate::app::state::View::Playlists;
    app.state.ui.selected_index = 0;

    app.handle_action(
        Action::Playlists(PlaylistAction::RenameSelectedPlaylist("Sunday".into())),
        &action_tx,
    )
    .await;

    assert_eq!(app.state.domain.playlists[0].name, "Sunday");
    let stored = app.playlists.list().expect("stored playlists");
    assert_eq!(stored[0].name, "Sunday");
}

#[tokio::test]
async fn pasted_json_import_persists_every_playlist_after_full_validation() {
    let (_temp, mut app) = test_app();
    app.state.ui.prompt = Some(crate::app::state::PromptState {
        purpose: PromptPurpose::ImportPlaylistJson,
        buffer: r#"{
          "version": 1,
          "playlists": [
            {"name":"One","tracks":[{"title":"A","channel":"Channel A","url":"https://youtu.be/aaaaaaaaaaa"}]},
            {"name":"Two","tracks":[{"title":"B","channel":"Channel B","url":"https://www.youtube.com/watch?v=bbbbbbbbbbb"}]}
          ]
        }"#
        .to_string(),
    });
    let (action_tx, _action_rx) = mpsc::channel(4);

    app.handle_action(Action::Playlists(PlaylistAction::PromptSubmit), &action_tx)
        .await;

    assert_eq!(app.playlists.list().expect("stored playlists").len(), 2);
    assert_eq!(app.state.domain.playlists.len(), 2);
    assert!(app.state.ui.prompt.is_none());
}

#[tokio::test]
async fn repeated_json_import_merges_by_name_and_video_id() {
    let (_temp, mut app) = test_app();
    let (action_tx, _action_rx) = mpsc::channel(4);
    let first = r#"{
      "version":1,
      "playlists":[{"name":"Same Name","tracks":[
        {"title":"Original","channel":"A","url":"https://youtu.be/aaaaaaaaaaa"}
      ]}]
    }"#;
    let second = r#"{
      "version":1,
      "playlists":[{"name":" same name ","tracks":[
        {"title":"Duplicate metadata","channel":"B","url":"https://youtu.be/aaaaaaaaaaa"},
        {"title":"New track","channel":"C","url":"https://youtu.be/bbbbbbbbbbb"}
      ]}]
    }"#;

    for json in [first, second] {
        app.state.ui.prompt = Some(crate::app::state::PromptState {
            purpose: PromptPurpose::ImportPlaylistJson,
            buffer: json.to_string(),
        });
        app.handle_action(Action::Playlists(PlaylistAction::PromptSubmit), &action_tx)
            .await;
    }

    let stored = app.playlists.list().expect("stored playlists");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].tracks.len(), 2);
    assert_eq!(stored[0].tracks[0].title, "Original");
    assert_eq!(app.state.domain.playlists.len(), 1);
}

#[tokio::test]
async fn playlist_editor_submit_persists_name_and_description() {
    let (_temp, mut app) = test_app();
    let playlist = Playlist::new("Before");
    let id = playlist.id.clone();
    app.playlists.save(&playlist).expect("seed playlist");
    app.state.domain.playlists.push(playlist);
    app.state.ui.selected_playlist = Some(0);
    app.state.ui.playlist_editor = Some(crate::app::state::PlaylistEditorState {
        name: " After ".to_string(),
        description: " Clear context ".to_string(),
        field: crate::app::state::PlaylistEditorField::Description,
    });
    let (action_tx, _action_rx) = mpsc::channel(2);

    app.handle_action(
        Action::Playlists(PlaylistAction::PlaylistEditorSubmit),
        &action_tx,
    )
    .await;

    let stored = app.playlists.get(&id).expect("stored playlist");
    assert_eq!(stored.name, "After");
    assert_eq!(stored.description, "Clear context");
    assert!(app.state.ui.playlist_editor.is_none());
}

#[tokio::test]
async fn invalid_pasted_json_remains_editable() {
    let (_temp, mut app) = test_app();
    app.state.ui.prompt = Some(crate::app::state::PromptState {
        purpose: PromptPurpose::ImportPlaylistJson,
        buffer: "not json".to_string(),
    });
    let (action_tx, _action_rx) = mpsc::channel(4);

    app.handle_action(Action::Playlists(PlaylistAction::PromptSubmit), &action_tx)
        .await;

    assert_eq!(
        app.state
            .ui
            .prompt
            .as_ref()
            .map(|prompt| prompt.buffer.as_str()),
        Some("not json")
    );
    assert!(
        app.state
            .ui
            .notification
            .as_ref()
            .is_some_and(|item| item.is_error)
    );
}
