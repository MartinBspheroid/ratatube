use super::*;

#[test]
fn history_top_mode_shows_play_counts() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::History;
    state.history_view_mode = ytm_tui::app::state::HistoryViewMode::Top;
    let dir = tempfile::tempdir().expect("tempdir");
    let mut history = HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
    for _ in 0..3 {
        history.record(HistoryEntry::from_track(
            &Track::new("a", "Favourite Mix", "DJ A"),
            None,
            PlaybackOutcome::Completed,
            3600,
        ));
    }
    history.record(HistoryEntry::from_track(
        &Track::new("b", "One-off", "DJ B"),
        None,
        PlaybackOutcome::Skipped,
        60,
    ));
    let out = render_to_string(&mut state, Some(&history), 110, 30);
    assert!(out.contains("3 plays · 3 tries"), "play count:\n{out}");
    assert!(out.contains("Favourite Mix"), "aggregated title:\n{out}");
    assert!(out.contains("3:00:00 total"), "total listened:\n{out}");
    assert!(out.contains("· TOP"), "mode label:\n{out}");
}

#[test]
fn queue_filter_bar_narrows_list() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Queue;
    state.queue.push(Track::new("a", "ISS006", "Skee Mask"));
    state.queue.push(Track::new("b", "Xtal", "Aphex Twin"));
    state.list_filter = Some("skee".to_string());
    state.visible_indices = Some(vec![0]);
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("/skee"), "filter bar:\n{out}");
    assert!(out.contains("1 of 2"), "match count:\n{out}");
    assert!(out.contains("ISS006"), "matching row:\n{out}");
    assert!(!out.contains("Xtal"), "filtered-out row:\n{out}");
}

#[test]
fn playlist_picker_modal_renders() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    state.playlists = vec![
        ytm_tui::playlists::Playlist::new("Techno Sets"),
        ytm_tui::playlists::Playlist::new("Ambient"),
    ];
    state.picker = Some(ytm_tui::app::state::PickerState {
        track: Track::new("t", "Song", "Artist"),
        filter: "tech".to_string(),
        selected: 0,
    });
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Add to playlist"), "modal title:\n{out}");
    assert!(
        out.contains("New playlist \"tech\""),
        "create entry:\n{out}"
    );
    assert!(out.contains("Techno Sets"), "matching playlist:\n{out}");
    assert!(
        !out.contains("Ambient  (0)"),
        "non-matching playlist hidden:\n{out}"
    );
}

#[test]
fn playlist_json_prompt_explains_paste_and_submit() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Playlists;
    state.prompt = Some(ytm_tui::app::state::PromptState {
        purpose: ytm_tui::app::state::PromptPurpose::ImportPlaylistJson,
        buffer: "{\n  \"version\": 1\n}".to_string(),
    });

    let out = render_to_string(&mut state, None, 100, 30);

    assert!(out.contains("Paste playlist JSON"), "modal title:\n{out}");
    assert!(out.contains("Enter import"), "submit instruction:\n{out}");
    assert!(out.contains("version"), "pasted content preview:\n{out}");
}

#[test]
fn playlist_editor_uses_stateful_table_inspector_and_edit_popup() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::PlaylistDetail;
    let mut playlist = ytm_tui::playlists::Playlist::new("Editable");
    playlist.description = "A useful description".to_string();
    playlist
        .tracks
        .push(ytm_tui::playlists::model::PlaylistTrack::from(&Track::new(
            "one",
            "First video",
            "First channel",
        )));
    playlist
        .tracks
        .push(ytm_tui::playlists::model::PlaylistTrack::from(&Track::new(
            "two",
            "Selected video",
            "Selected channel",
        )));
    state.playlists.push(playlist);
    state.selected_playlist = Some(0);
    state.selected_index = 1;

    let browse = render_to_string(&mut state, None, 120, 32);
    assert!(
        browse.contains("PLAYLIST EDITOR"),
        "editor title:\n{browse}"
    );
    assert!(browse.contains("SELECTED TRACK"), "inspector:\n{browse}");
    assert!(browse.contains("Selected video"), "selected row:\n{browse}");
    assert!(
        browse.contains("e edit details"),
        "footer keeps the editor hints after the BROWSE MODE row removal:\n{browse}"
    );

    state.playlist_editor = Some(ytm_tui::app::state::PlaylistEditorState {
        name: "Edited name".to_string(),
        description: "Edited description".to_string(),
        field: ytm_tui::app::state::PlaylistEditorField::Name,
    });
    let editing = render_to_string(&mut state, None, 120, 32);
    assert!(editing.contains("Edit playlist"), "popup title:\n{editing}");
    assert!(editing.contains("Edited name"), "name field:\n{editing}");
    assert!(
        editing.contains("Edited description"),
        "description field:\n{editing}"
    );
    assert!(
        editing.contains("Tab next field"),
        "field instructions:\n{editing}"
    );
}

#[test]
fn visual_dump_help_and_modal() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Help;
    let out = render_to_string(&mut state, None, 100, 34);
    println!("\n{out}");
    assert!(out.contains("Seek 5 seconds"));

    let mut state = AppState::new();
    state.import = Some(ytm_tui::app::state::ImportState::Review {
        summary: ytm_tui::playlists::import::ImportSummary {
            remote_title: "Popular Music Videos".to_string(),
            remote_url: "https://...".to_string(),
            total_entries: 205,
            imported: 183,
            deleted: 2,
            private: 3,
            unavailable: 5,
            duplicates: 12,
            missing_id: 0,
            missing_title: 0,
        },
        playlist: Box::new(ytm_tui::playlists::Playlist::new("Popular Music Videos")),
    });
    let out = render_to_string(&mut state, None, 100, 30);
    println!("\n{out}");
    assert!(out.contains("183"));
}
