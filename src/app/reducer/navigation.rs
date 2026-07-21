//! Navigation-family coordinator: routes UI-only transitions to the UI
//! reducer, applies domain search transitions on [`DomainState`], and keeps
//! the residual cross-half glue explicit in one place.

use crate::app::action::NavigationAction;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, Focus};
use crate::media::Track;
use crate::media::search::SearchState;

/// Outcome of accepting or rejecting a completed search.
enum SearchOutcome {
    Accepted,
    AcceptedEmpty,
    Stale,
}

/// Reduce navigation-domain state transitions.
pub(super) fn reduce(state: &mut AppState, action: NavigationAction) -> Vec<Effect> {
    match action {
        NavigationAction::OpenTrackContext
        | NavigationAction::CloseTrackContext
        | NavigationAction::MoveTrackContext(_)
        | NavigationAction::SubmitTrackContext
        | NavigationAction::ShowTrackDetails(_)
        | NavigationAction::CloseTrackDetails => {
            super::ui::modals::reduce_track_context(&mut state.ui, &state.domain, action)
        }
        NavigationAction::ClearSearch => {
            state.ui.search_input.clear();
            state.domain.search = SearchState::Idle;
            Vec::new()
        }
        NavigationAction::SubmitSearch(query) => {
            if query.trim().is_empty() {
                return Vec::new();
            }
            state.ui.focus = Focus::Content;
            submit_search(&mut state.domain, query)
        }
        NavigationAction::SubmitExactVideo(url) => {
            state.ui.focus = Focus::Content;
            submit_exact_video(&mut state.domain, url)
        }
        NavigationAction::SearchCompleted { generation, tracks } => {
            match search_completed(&mut state.domain, generation, tracks) {
                SearchOutcome::AcceptedEmpty => {
                    state.notify("No results", false);
                    state.ui.selected_index = 0;
                }
                SearchOutcome::Accepted => state.ui.selected_index = 0,
                SearchOutcome::Stale => {}
            }
            Vec::new()
        }
        NavigationAction::SearchFailed {
            generation,
            message,
        } => {
            search_failed(&mut state.domain, generation, message);
            Vec::new()
        }
        // UI-only navigation plus the service-owned no-op intents.
        other => super::ui::navigation::reduce(&mut state.ui, &state.domain, other),
    }
}

/// Begin a supervised search, superseding any in-flight generation.
fn submit_search(domain: &mut DomainState, query: String) -> Vec<Effect> {
    domain.search_generation += 1;
    let generation = domain.search_generation;
    domain.search = SearchState::Searching {
        query: query.clone(),
        generation,
    };
    vec![Effect::RunSearch { query, generation }]
}

/// Begin a supervised exact-video fetch, superseding any in-flight generation.
fn submit_exact_video(domain: &mut DomainState, url: String) -> Vec<Effect> {
    domain.search_generation += 1;
    let generation = domain.search_generation;
    domain.search = SearchState::Searching {
        query: url.clone(),
        generation,
    };
    vec![Effect::RunExactVideo { url, generation }]
}

/// Accept results only for the current generation (PRD 15).
fn search_completed(
    domain: &mut DomainState,
    generation: u64,
    tracks: Vec<Track>,
) -> SearchOutcome {
    if generation != domain.search_generation {
        return SearchOutcome::Stale;
    }
    let query = domain.search.query().to_string();
    let empty = tracks.is_empty();
    domain.search = SearchState::Results { query, tracks };
    if empty {
        SearchOutcome::AcceptedEmpty
    } else {
        SearchOutcome::Accepted
    }
}

/// Record a failed search only for the current generation.
fn search_failed(domain: &mut DomainState, generation: u64, message: String) {
    if generation == domain.search_generation {
        let query = domain.search.query().to_string();
        domain.search = SearchState::Failed { query, message };
    }
}
