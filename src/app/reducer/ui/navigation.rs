//! View navigation, help, search-input editing, and selection transitions.

use crate::app::action::NavigationAction;
use crate::app::reducer::Effect;
use crate::app::state::{DomainState, Focus, UiState, View};

/// Reduce UI-only navigation transitions. Domain-facing search variants are
/// handled by the navigation coordinator before reaching this function.
pub(in crate::app::reducer) fn reduce(
    ui: &mut UiState,
    domain: &DomainState,
    action: NavigationAction,
) -> Vec<Effect> {
    match action {
        NavigationAction::Navigate(view) => navigate(ui, domain, view),
        NavigationAction::OpenHelp => {
            if ui.view != View::Help {
                ui.help_return_view = ui.view;
            }
            ui.view = View::Help;
            ui.help_scroll = 0;
            ui.focus = Focus::Content;
        }
        NavigationAction::CloseHelp => {
            ui.view = ui.help_return_view;
            ui.help_scroll = 0;
        }
        NavigationAction::ScrollHelp(delta) => {
            if ui.view == View::Help {
                ui.help_scroll = ui.help_scroll.saturating_add_signed(delta as i16);
            }
        }
        NavigationAction::NextView => navigate(ui, domain, ui.view.next_tab()),
        NavigationAction::PreviousView => navigate(ui, domain, ui.view.prev_tab()),
        NavigationAction::CycleHomeSection(delta) => {
            if ui.view == View::Home {
                ui.home_section = ui.home_section.cycled(delta);
                ui.selected_index = 0;
                ui.reset_list(domain);
            }
        }
        NavigationAction::Quit => {
            ui.running = false;
            return vec![Effect::PersistQueue, Effect::QuitMpv, Effect::Exit];
        }
        NavigationAction::SearchInput(c) => {
            if ui.focus == Focus::SearchInput {
                ui.search_input.push(c);
            }
        }
        NavigationAction::SearchBackspace => {
            if ui.focus == Focus::SearchInput {
                ui.search_input.pop();
            }
        }
        NavigationAction::ToggleSearchDetail => {
            if ui.view == View::Search
                && crate::ui::layout::Breakpoint::from_width(ui.screen_area.width)
                    == crate::ui::layout::Breakpoint::Narrow
            {
                ui.search_detail_open = !ui.search_detail_open;
            }
        }
        NavigationAction::SelectNext => {
            let len = ui.active_list_len(domain);
            if len > 0 {
                ui.selected_index = (ui.selected_index + 1).min(len - 1);
            }
        }
        NavigationAction::SelectPrevious => {
            ui.selected_index = ui.selected_index.saturating_sub(1);
        }
        // Service-owned intents (browser, channel) and domain search variants
        // are no-ops here, exactly as in the pre-split catch-all.
        _ => {}
    }
    Vec::new()
}

fn navigate(ui: &mut UiState, domain: &DomainState, view: View) {
    ui.view = view;
    ui.selected_index = 0;
    ui.focus = Focus::Content;
    ui.list_filter = None;
    ui.visible_indices = None;
    ui.search_detail_open = false;
    ui.reset_list(domain);
}
