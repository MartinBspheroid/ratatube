use super::*;

#[test]
fn help_is_scrollable_at_minimum_supported_viewport() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Help;
    let first = render_to_string(&mut state, None, 80, 24);
    assert!(first.contains("J/K OR PGUP/PGDN SCROLL"));
    state.help_scroll = u16::MAX;
    let last = render_to_string(&mut state, None, 80, 24);
    assert!(last.contains("Message log"), "last help page:\n{last}");
    assert!(last.contains("Quit"), "last help page:\n{last}");
}

#[test]
fn renderer_exposes_exact_search_result_hit_rows() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    state.search = SearchState::Results {
        query: "query".to_string(),
        tracks: vec![Track::new("id", "title", "artist")],
    };
    let _ = render_to_string(&mut state, None, 80, 24);
    assert!(state.list_hit_area.y > state.main_area.y + 3);
    assert!(state.list_hit_area.height > 0);
}
