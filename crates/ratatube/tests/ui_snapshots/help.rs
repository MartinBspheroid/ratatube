use super::*;

#[test]
fn help_is_scrollable_at_minimum_supported_viewport() {
    let mut state = AppState::new();
    state.ui.view = ratatube::app::state::View::Help;
    let first = render_to_string(&mut state, None, 80, 24);
    assert!(first.contains("HELP"), "help title:\n{first}");
    assert!(
        first.contains("j/k scroll"),
        "scroll hint lives in the footer, not the title:\n{first}"
    );
    state.ui.help_scroll = u16::MAX;
    let last = render_to_string(&mut state, None, 80, 24);
    assert!(last.contains("Message log"), "last help page:\n{last}");
    assert!(last.contains("Quit"), "last help page:\n{last}");
}

#[test]
fn renderer_exposes_exact_search_result_hit_rows() {
    let mut state = AppState::new();
    state.ui.view = ratatube::app::state::View::Search;
    state.domain.search = SearchState::Results {
        query: "query".to_string(),
        tracks: vec![Track::new("id", "title", "artist")],
    };
    let _ = render_to_string(&mut state, None, 80, 24);
    assert!(state.ui.list_hit_area.y > state.ui.main_area.y + 3);
    assert!(state.ui.list_hit_area.height > 0);
}

#[test]
fn queue_and_history_footers_document_uppercase_clear_shortcut() {
    for view in [
        ratatube::app::state::View::Queue,
        ratatube::app::state::View::History,
    ] {
        let mut state = AppState::new();
        state.ui.view = view;
        let rendered = render_to_string(&mut state, None, 100, 24);

        assert!(
            rendered.contains(" C clear"),
            "missing uppercase clear hint for {view:?}:\n{rendered}"
        );
    }
}
