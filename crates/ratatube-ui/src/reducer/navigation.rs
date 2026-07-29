//! View navigation, help, and selection transitions.

use crate::state::{DomainState, Focus, UiState, View};
use ratatube_domain::commands::UiMsg;
use ratatube_domain::effect::Effect;

/// Reduce view, help, and selection presentation messages.
pub fn reduce(ui: &mut UiState, domain: &DomainState, msg: UiMsg) -> Vec<Effect> {
    match msg {
        UiMsg::Navigate(view) => navigate(ui, domain, view),
        UiMsg::OpenHelp => {
            if ui.view != View::Help {
                ui.help_return_view = ui.view;
            }
            ui.view = View::Help;
            ui.help_scroll = 0;
            ui.focus = Focus::Content;
        }
        UiMsg::CloseHelp => {
            ui.view = ui.help_return_view;
            ui.help_scroll = 0;
        }
        UiMsg::ScrollHelp(delta) => {
            if ui.view == View::Help {
                ui.help_scroll = ui.help_scroll.saturating_add_signed(delta as i16);
            }
        }
        UiMsg::NextView => navigate(ui, domain, ui.view.next_tab()),
        UiMsg::PreviousView => navigate(ui, domain, ui.view.prev_tab()),
        UiMsg::CycleHomeSection(delta) => {
            if ui.view == View::Home {
                ui.home_section = ui.home_section.cycled(delta);
                ui.selected_index = 0;
                ui.reset_list(domain);
            }
        }
        UiMsg::ToggleSearchDetail => {
            if ui.view == View::Search
                && crate::render::layout::Breakpoint::from_width(ui.screen_area.width)
                    == crate::render::layout::Breakpoint::Narrow
            {
                ui.search_detail_open = !ui.search_detail_open;
            }
        }
        UiMsg::SelectNext => {
            let len = ui.active_list_len(domain);
            if len > 0 {
                ui.selected_index = (ui.selected_index + 1).min(len - 1);
            }
        }
        UiMsg::SelectPrevious => {
            ui.selected_index = ui.selected_index.saturating_sub(1);
        }
        // Settings and presentation messages are dispatched to their own
        // reducers before reaching this function.
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
