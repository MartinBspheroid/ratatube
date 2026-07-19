use super::*;

#[tokio::test]
async fn pasted_json_import_persists_every_playlist_after_full_validation() {
    let (_temp, mut app) = test_app();
    app.state.prompt = Some(crate::app::state::PromptState {
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
    assert_eq!(app.state.playlists.len(), 2);
    assert!(app.state.prompt.is_none());
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
        app.state.prompt = Some(crate::app::state::PromptState {
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
    assert_eq!(app.state.playlists.len(), 1);
}

#[tokio::test]
async fn playlist_editor_submit_persists_name_and_description() {
    let (_temp, mut app) = test_app();
    let playlist = Playlist::new("Before");
    let id = playlist.id.clone();
    app.playlists.save(&playlist).expect("seed playlist");
    app.state.playlists.push(playlist);
    app.state.selected_playlist = Some(0);
    app.state.playlist_editor = Some(crate::app::state::PlaylistEditorState {
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
    assert!(app.state.playlist_editor.is_none());
}

#[tokio::test]
async fn invalid_pasted_json_remains_editable() {
    let (_temp, mut app) = test_app();
    app.state.prompt = Some(crate::app::state::PromptState {
        purpose: PromptPurpose::ImportPlaylistJson,
        buffer: "not json".to_string(),
    });
    let (action_tx, _action_rx) = mpsc::channel(4);

    app.handle_action(Action::Playlists(PlaylistAction::PromptSubmit), &action_tx)
        .await;

    assert_eq!(
        app.state
            .prompt
            .as_ref()
            .map(|prompt| prompt.buffer.as_str()),
        Some("not json")
    );
    assert!(
        app.state
            .notification
            .as_ref()
            .is_some_and(|item| item.is_error)
    );
}
