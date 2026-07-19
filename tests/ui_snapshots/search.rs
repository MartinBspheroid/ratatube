use super::*;

#[test]
fn empty_search_state_shows_prompt() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(
        out.contains("Type a query and press Enter"),
        "search hint:\n{out}"
    );
    assert!(out.contains("[SEARCH] Search"), "header tabs:\n{out}");
    assert!(out.contains("mpv down"), "status:\n{out}");
}

#[test]
fn empty_queue_state() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Queue;
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Queue is empty"), "empty queue:\n{out}");
}

#[test]
fn search_results_render_sanitized() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    let mut track = Track::new("abc123", "Nice Song", "Good Artist");
    track.title = "Bad\u{1b}[2JTitle".to_string();
    state.search = SearchState::Results {
        query: "q".to_string(),
        tracks: vec![track],
    };
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Good Artist"), "artist:\n{out}");
    assert!(!out.contains('\u{1b}'), "no control chars leak:\n{out:?}");
}

#[test]
fn search_table_has_membership_icons_and_minimal_selected_metadata() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    let mut track = Track::new("selected-id", "Selected width-safe", "Selected channel");
    track.duration_seconds = Some(213);
    state.search = SearchState::Results {
        query: "selected".to_string(),
        tracks: vec![track.clone()],
    };
    let before = render_to_string(&mut state, None, 150, 40);
    for label in ["SELECTED", "Duration", "Channel"] {
        assert!(before.contains(label), "missing {label}:\n{before}");
    }
    assert!(
        before.contains("CHANNEL"),
        "search column semantics:\n{before}"
    );
    assert!(
        !before.contains("ARTIST"),
        "channel mislabeled as artist:\n{before}"
    );
    for removed in ["Source", "ID", "In queue", "+"] {
        assert!(
            !before.contains(removed),
            "removed field {removed}:\n{before}"
        );
    }
    assert!(
        !before.contains("QUICK ACTIONS"),
        "footer is the only shortcut surface:\n{before}"
    );
    for deferred in ["Published", "views", "likes", "Bitrate", "Format", "Tags"] {
        assert!(
            !before.contains(deferred),
            "deferred row {deferred}:\n{before}"
        );
    }

    state.queue.push(track.clone());
    let mut playlist = ytm_tui::playlists::Playlist::new("Membership");
    playlist
        .tracks
        .push(ytm_tui::playlists::model::PlaylistTrack::from(&track));
    state.playlists.push(playlist);
    let after = render_to_string(&mut state, None, 150, 40);
    let result_row = after
        .lines()
        .find(|line| line.contains("Selected width-safe"))
        .expect("selected result row");
    assert!(result_row.contains("[QUEUE]"), "queued icon:\n{after}");
    assert!(result_row.contains('*'), "playlist dot:\n{after}");
}

#[test]
fn narrow_search_detail_is_a_local_modal() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    state.search = SearchState::Results {
        query: "narrow".to_string(),
        tracks: vec![Track::new("narrow-id", "Narrow selected", "Channel")],
    };
    let list = render_to_string(&mut state, None, 80, 24);
    assert!(
        !list.contains("QUICK ACTIONS"),
        "detail hidden by default:\n{list}"
    );
    assert!(
        list.lines()
            .last()
            .unwrap_or_default()
            .contains("i details")
    );

    state.search_detail_open = true;
    let modal = render_to_string(&mut state, None, 80, 24);
    assert!(modal.contains("SELECTED"), "modal detail:\n{modal}");
    assert!(
        !modal.contains("QUICK ACTIONS"),
        "modal has no duplicate actions:\n{modal}"
    );
}
