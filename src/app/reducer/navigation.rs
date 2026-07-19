//! Navigation, search, and selection state transitions.

use crate::app::action::{Action, NavigationAction};
use crate::app::reducer::{Effect, reduce as reduce_action};
use crate::app::state::{AppState, Focus, View};
use crate::media::search::SearchState;

/// Reduce navigation-domain state transitions.
pub(super) fn reduce(state: &mut AppState, action: NavigationAction) -> Vec<Effect> {
    if matches!(
        &action,
        NavigationAction::OpenTrackContext
            | NavigationAction::CloseTrackContext
            | NavigationAction::MoveTrackContext(_)
            | NavigationAction::SubmitTrackContext
            | NavigationAction::ShowTrackDetails(_)
            | NavigationAction::CloseTrackDetails
    ) {
        return super::track_context::reduce(state, action);
    }
    match Action::Navigation(action) {
        Action::Navigation(NavigationAction::Navigate(view)) => {
            state.view = view;
            state.selected_index = 0;
            state.focus = Focus::Content;
            state.list_filter = None;
            state.visible_indices = None;
            state.search_detail_open = false;
            state.reset_list();
        }
        Action::Navigation(NavigationAction::OpenHelp) => {
            if state.view != View::Help {
                state.help_return_view = state.view;
            }
            state.view = View::Help;
            state.help_scroll = 0;
            state.focus = Focus::Content;
        }
        Action::Navigation(NavigationAction::CloseHelp) => {
            state.view = state.help_return_view;
            state.help_scroll = 0;
        }
        Action::Navigation(NavigationAction::ScrollHelp(delta)) => {
            if state.view == View::Help {
                state.help_scroll = state.help_scroll.saturating_add_signed(delta as i16);
            }
        }
        Action::Navigation(NavigationAction::NextView) => {
            return reduce_action(
                state,
                Action::Navigation(NavigationAction::Navigate(state.view.next_tab())),
            );
        }
        Action::Navigation(NavigationAction::PreviousView) => {
            return reduce_action(
                state,
                Action::Navigation(NavigationAction::Navigate(state.view.prev_tab())),
            );
        }
        Action::Navigation(NavigationAction::CycleHomeSection(delta)) => {
            if state.view == View::Home {
                state.home_section = state.home_section.cycled(delta);
                state.selected_index = 0;
                state.reset_list();
            }
        }
        Action::Navigation(NavigationAction::Quit) => {
            state.running = false;
            return vec![Effect::PersistQueue, Effect::QuitMpv, Effect::Exit];
        }

        // --- Search input -------------------------------------------------
        Action::Navigation(NavigationAction::SearchInput(c)) => {
            if state.focus == Focus::SearchInput {
                state.search_input.push(c);
            }
        }
        Action::Navigation(NavigationAction::SearchBackspace) => {
            if state.focus == Focus::SearchInput {
                state.search_input.pop();
            }
        }
        Action::Navigation(NavigationAction::ClearSearch) => {
            state.search_input.clear();
            state.search = SearchState::Idle;
        }
        Action::Navigation(NavigationAction::ToggleSearchDetail) => {
            if state.view == View::Search
                && crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                    == crate::ui::layout::Breakpoint::Narrow
            {
                state.search_detail_open = !state.search_detail_open;
            }
        }
        // Resolved by the app layer because it needs the selected track and
        // an operating-system process boundary.
        Action::Navigation(NavigationAction::OpenInBrowser) => {}
        // Task 5 consumes this typed intent and owns channel navigation.
        Action::Navigation(NavigationAction::VisitChannel(_)) => {}
        Action::Navigation(NavigationAction::SubmitSearch(query)) => {
            if query.trim().is_empty() {
                return Vec::new();
            }
            state.search_generation += 1;
            let generation = state.search_generation;
            state.search = SearchState::Searching {
                query: query.clone(),
                generation,
            };
            state.focus = Focus::Content;
            return vec![Effect::RunSearch { query, generation }];
        }
        Action::Navigation(NavigationAction::SubmitExactVideo(url)) => {
            state.search_generation += 1;
            let generation = state.search_generation;
            state.search = SearchState::Searching {
                query: url.clone(),
                generation,
            };
            state.focus = Focus::Content;
            return vec![Effect::RunExactVideo { url, generation }];
        }
        Action::Navigation(NavigationAction::SearchCompleted { generation, tracks }) => {
            // Discard results from superseded searches (PRD 15).
            if generation == state.search_generation {
                let query = state.search.query().to_string();
                if tracks.is_empty() {
                    state.notify("No results", false);
                }
                state.search = SearchState::Results { query, tracks };
                state.selected_index = 0;
            }
        }
        Action::Navigation(NavigationAction::SearchFailed {
            generation,
            message,
        }) => {
            if generation == state.search_generation {
                let query = state.search.query().to_string();
                state.search = SearchState::Failed { query, message };
            }
        }

        // --- Selection ----------------------------------------------------
        Action::Navigation(NavigationAction::SelectNext) => {
            let len = state.active_list_len();
            if len > 0 {
                state.selected_index = (state.selected_index + 1).min(len - 1);
            }
        }
        Action::Navigation(NavigationAction::SelectPrevious) => {
            state.selected_index = state.selected_index.saturating_sub(1);
        }
        _ => {}
    }
    Vec::new()
}
