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
            state.ui.view = view;
            state.ui.selected_index = 0;
            state.ui.focus = Focus::Content;
            state.ui.list_filter = None;
            state.ui.visible_indices = None;
            state.ui.search_detail_open = false;
            state.reset_list();
        }
        Action::Navigation(NavigationAction::OpenHelp) => {
            if state.ui.view != View::Help {
                state.ui.help_return_view = state.ui.view;
            }
            state.ui.view = View::Help;
            state.ui.help_scroll = 0;
            state.ui.focus = Focus::Content;
        }
        Action::Navigation(NavigationAction::CloseHelp) => {
            state.ui.view = state.ui.help_return_view;
            state.ui.help_scroll = 0;
        }
        Action::Navigation(NavigationAction::ScrollHelp(delta)) => {
            if state.ui.view == View::Help {
                state.ui.help_scroll = state.ui.help_scroll.saturating_add_signed(delta as i16);
            }
        }
        Action::Navigation(NavigationAction::NextView) => {
            return reduce_action(
                state,
                Action::Navigation(NavigationAction::Navigate(state.ui.view.next_tab())),
            );
        }
        Action::Navigation(NavigationAction::PreviousView) => {
            return reduce_action(
                state,
                Action::Navigation(NavigationAction::Navigate(state.ui.view.prev_tab())),
            );
        }
        Action::Navigation(NavigationAction::CycleHomeSection(delta)) => {
            if state.ui.view == View::Home {
                state.ui.home_section = state.ui.home_section.cycled(delta);
                state.ui.selected_index = 0;
                state.reset_list();
            }
        }
        Action::Navigation(NavigationAction::Quit) => {
            state.ui.running = false;
            return vec![Effect::PersistQueue, Effect::QuitMpv, Effect::Exit];
        }

        // --- Search input -------------------------------------------------
        Action::Navigation(NavigationAction::SearchInput(c)) => {
            if state.ui.focus == Focus::SearchInput {
                state.ui.search_input.push(c);
            }
        }
        Action::Navigation(NavigationAction::SearchBackspace) => {
            if state.ui.focus == Focus::SearchInput {
                state.ui.search_input.pop();
            }
        }
        Action::Navigation(NavigationAction::ClearSearch) => {
            state.ui.search_input.clear();
            state.domain.search = SearchState::Idle;
        }
        Action::Navigation(NavigationAction::ToggleSearchDetail) => {
            if state.ui.view == View::Search
                && crate::ui::layout::Breakpoint::from_width(state.ui.screen_area.width)
                    == crate::ui::layout::Breakpoint::Narrow
            {
                state.ui.search_detail_open = !state.ui.search_detail_open;
            }
        }
        // Resolved by the app layer because it needs the selected track and
        // an operating-system process boundary.
        Action::Navigation(NavigationAction::OpenInBrowser) => {}
        // The service layer consumes these typed intents and owns channel navigation.
        Action::Navigation(
            NavigationAction::VisitChannel(_)
            | NavigationAction::ChannelResolved { .. }
            | NavigationAction::ChannelPageLoaded { .. }
            | NavigationAction::LoadMoreChannel
            | NavigationAction::RetryChannel
            | NavigationAction::BackFromChannel,
        ) => {}
        Action::Navigation(NavigationAction::SubmitSearch(query)) => {
            if query.trim().is_empty() {
                return Vec::new();
            }
            state.domain.search_generation += 1;
            let generation = state.domain.search_generation;
            state.domain.search = SearchState::Searching {
                query: query.clone(),
                generation,
            };
            state.ui.focus = Focus::Content;
            return vec![Effect::RunSearch { query, generation }];
        }
        Action::Navigation(NavigationAction::SubmitExactVideo(url)) => {
            state.domain.search_generation += 1;
            let generation = state.domain.search_generation;
            state.domain.search = SearchState::Searching {
                query: url.clone(),
                generation,
            };
            state.ui.focus = Focus::Content;
            return vec![Effect::RunExactVideo { url, generation }];
        }
        Action::Navigation(NavigationAction::SearchCompleted { generation, tracks }) => {
            // Discard results from superseded searches (PRD 15).
            if generation == state.domain.search_generation {
                let query = state.domain.search.query().to_string();
                if tracks.is_empty() {
                    state.notify("No results", false);
                }
                state.domain.search = SearchState::Results { query, tracks };
                state.ui.selected_index = 0;
            }
        }
        Action::Navigation(NavigationAction::SearchFailed {
            generation,
            message,
        }) => {
            if generation == state.domain.search_generation {
                let query = state.domain.search.query().to_string();
                state.domain.search = SearchState::Failed { query, message };
            }
        }

        // --- Selection ----------------------------------------------------
        Action::Navigation(NavigationAction::SelectNext) => {
            let len = state.active_list_len();
            if len > 0 {
                state.ui.selected_index = (state.ui.selected_index + 1).min(len - 1);
            }
        }
        Action::Navigation(NavigationAction::SelectPrevious) => {
            state.ui.selected_index = state.ui.selected_index.saturating_sub(1);
        }
        _ => {}
    }
    Vec::new()
}
