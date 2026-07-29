//! Navigation-family coordinator: routes UI-only transitions to the UI
//! reducer, applies domain search transitions on [`DomainState`], and keeps
//! the residual cross-half glue explicit in one place.

use crate::app::action::NavigationAction;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, Focus};
use crate::media::Track;
use crate::media::search::SearchState;
use ratatube_ui::reducer::modals::TrackContextMsg;

/// Apply one narrowed context-menu message to the UI half.
fn context_menu(state: &mut AppState, msg: TrackContextMsg) -> Vec<Effect> {
    super::ui::modals::reduce_context_menu(&mut state.ui, &state.domain, msg)
}

/// Outcome of accepting or rejecting a completed search.
enum SearchOutcome {
    Accepted,
    AcceptedEmpty,
    Stale,
}

/// Reduce navigation-domain state transitions.
pub(super) fn reduce(state: &mut AppState, action: NavigationAction) -> Vec<Effect> {
    match action {
        // Each context-menu action names the narrowed message it maps to, so
        // routing a non-context action here cannot compile.
        NavigationAction::OpenTrackContext => context_menu(state, TrackContextMsg::Open),
        NavigationAction::CloseTrackContext => context_menu(state, TrackContextMsg::Close),
        NavigationAction::MoveTrackContext(delta) => {
            context_menu(state, TrackContextMsg::Move(delta))
        }
        NavigationAction::SubmitTrackContext => context_menu(state, TrackContextMsg::Submit),
        NavigationAction::ShowTrackDetails(track) => {
            context_menu(state, TrackContextMsg::ShowDetails(track))
        }
        NavigationAction::CloseTrackDetails => context_menu(state, TrackContextMsg::CloseDetails),
        NavigationAction::Quit => {
            state.ui.running = false;
            vec![Effect::PersistQueue, Effect::QuitMpv, Effect::Exit]
        }
        NavigationAction::SearchInput(c) => {
            if state.ui.focus == Focus::SearchInput {
                state.ui.search_input.push(c);
            }
            Vec::new()
        }
        NavigationAction::SearchBackspace => {
            if state.ui.focus == Focus::SearchInput {
                state.ui.search_input.pop();
            }
            Vec::new()
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
        // Service-owned intents (browser, clipboard, channel) reduce to
        // nothing here; the service layer handles them after reduce.
        NavigationAction::OpenInBrowser
        | NavigationAction::ExternalCommandCompleted { .. }
        | NavigationAction::VisitChannel(_)
        | NavigationAction::ChannelResolved { .. }
        | NavigationAction::ChannelPageLoaded { .. }
        | NavigationAction::LoadMoreChannel
        | NavigationAction::RetryChannel
        | NavigationAction::BackFromChannel => Vec::new(),
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
