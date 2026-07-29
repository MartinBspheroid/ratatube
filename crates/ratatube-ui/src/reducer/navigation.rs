//! View navigation, help, and selection transitions.
//!
//! Each transition takes only the state and payload it needs; the parent
//! [`crate::reducer::reduce`] owns the wildcard-free routing, so no
//! navigation transition can be reached by a message it does not own.

use crate::state::{DomainState, Focus, UiState, View};

/// Enter `view`, resetting selection, focus, and any active list filter.
pub(super) fn navigate(ui: &mut UiState, domain: &DomainState, view: View) {
    ui.view = view;
    ui.selected_index = 0;
    ui.focus = Focus::Content;
    ui.list_filter = None;
    ui.visible_indices = None;
    ui.search_detail_open = false;
    ui.reset_list(domain);
}

/// Move to the next tab in the view order.
pub(super) fn next_view(ui: &mut UiState, domain: &DomainState) {
    navigate(ui, domain, ui.view.next_tab());
}

/// Move to the previous tab in the view order.
pub(super) fn previous_view(ui: &mut UiState, domain: &DomainState) {
    navigate(ui, domain, ui.view.prev_tab());
}

/// Open help while remembering the view that opened it.
pub(super) fn open_help(ui: &mut UiState) {
    if ui.view != View::Help {
        ui.help_return_view = ui.view;
    }
    ui.view = View::Help;
    ui.help_scroll = 0;
    ui.focus = Focus::Content;
}

/// Return from help to the view that opened it.
pub(super) fn close_help(ui: &mut UiState) {
    ui.view = ui.help_return_view;
    ui.help_scroll = 0;
}

/// Scroll the help document by signed rows.
pub(super) fn scroll_help(ui: &mut UiState, delta: i32) {
    if ui.view == View::Help {
        ui.help_scroll = ui.help_scroll.saturating_add_signed(delta as i16);
    }
}

/// Move Home section focus forward/backward.
pub(super) fn cycle_home_section(ui: &mut UiState, domain: &DomainState, delta: i32) {
    if ui.view == View::Home {
        ui.home_section = ui.home_section.cycled(delta);
        ui.selected_index = 0;
        ui.reset_list(domain);
    }
}

/// Toggle the selected-result detail overlay on narrow Search layouts.
pub(super) fn toggle_search_detail(ui: &mut UiState) {
    if ui.view == View::Search
        && crate::render::layout::Breakpoint::from_width(ui.screen_area.width)
            == crate::render::layout::Breakpoint::Narrow
    {
        ui.search_detail_open = !ui.search_detail_open;
    }
}

/// Move the selection down, clamped to the active list.
pub(super) fn select_next(ui: &mut UiState, domain: &DomainState) {
    let len = ui.active_list_len(domain);
    if len > 0 {
        ui.selected_index = (ui.selected_index + 1).min(len - 1);
    }
}

/// Move the selection up, clamped at the top of the list.
pub(super) fn select_previous(ui: &mut UiState) {
    ui.selected_index = ui.selected_index.saturating_sub(1);
}
