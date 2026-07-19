use super::*;

#[test]
fn superseded_import_completion_is_discarded() {
    let mut state = AppState::new();
    let mut operations = OperationRegistry::default();
    let stale = operations.start(OperationKind::Import);
    let current = operations.start(OperationKind::Import);
    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::ImportStarted {
            operation_id: current.id(),
            url: "https://example.invalid/current".to_string(),
        }),
    );
    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::ImportCompleted {
            operation_id: stale.id(),
            url: "https://example.invalid/stale".to_string(),
            title: "stale".to_string(),
            remote_id: None,
            tracks: vec![track("stale")],
            rejections: crate::media::yt_dlp::ImportRejections::default(),
        }),
    );

    assert!(matches!(
        state.import,
        Some(crate::app::state::ImportState::Fetching {
            operation_id,
            ..
        }) if operation_id == current.id()
    ));
}

#[test]
fn current_import_failure_has_a_terminal_error_state() {
    let mut state = AppState::new();
    let mut operations = OperationRegistry::default();
    let current = operations.start(OperationKind::Import);
    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::ImportStarted {
            operation_id: current.id(),
            url: "https://example.invalid/current".to_string(),
        }),
    );
    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::ImportFailed {
            operation_id: current.id(),
            url: "https://example.invalid/current".to_string(),
            message: "offline".to_string(),
        }),
    );

    assert!(matches!(
        state.import,
        Some(crate::app::state::ImportState::Failed { ref message, .. }) if message == "offline"
    ));
}

#[test]
fn prompt_paste_keeps_multiline_json_as_one_input() {
    let mut state = AppState::new();
    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::OpenPrompt(
            crate::app::state::PromptPurpose::ImportPlaylistJson,
        )),
    );

    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::PromptPaste(
            "{\n  \"version\": 1\n}".to_string(),
        )),
    );

    assert_eq!(
        state.prompt.as_ref().map(|prompt| prompt.buffer.as_str()),
        Some("{\n  \"version\": 1\n}")
    );
}

#[test]
fn playlist_editor_copies_metadata_and_switches_fields() {
    let mut state = AppState::new();
    let mut playlist = crate::playlists::Playlist::new("Original");
    playlist.description = "Existing description".to_string();
    state.playlists.push(playlist);
    state.selected_playlist = Some(0);
    state.view = View::PlaylistDetail;

    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::OpenPlaylistEditor),
    );
    assert_eq!(
        state
            .playlist_editor
            .as_ref()
            .map(|editor| editor.name.as_str()),
        Some("Original")
    );

    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::PlaylistEditorNextField),
    );
    reduce(
        &mut state,
        Action::Playlists(PlaylistAction::PlaylistEditorInput('!')),
    );

    let editor = state.playlist_editor.as_ref().expect("editor open");
    assert_eq!(
        editor.field,
        crate::app::state::PlaylistEditorField::Description
    );
    assert_eq!(editor.description, "Existing description!");
}
