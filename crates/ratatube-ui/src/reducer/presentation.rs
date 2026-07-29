//! Presentation-mode toggles and notification transitions.
//!
//! Each transition takes only the state and payload it needs; the parent
//! [`crate::reducer::reduce`] owns the wildcard-free routing, so no
//! presentation transition can be reached by a message it does not own.

use crate::state::{DomainState, HistoryViewMode, PlayingPane, UiState, View};
use ratatube_domain::action::HistoryAction;
use ratatube_domain::effect::Effect;

/// Reduce service-notification transitions that remain history-family.
///
/// Wildcard-free on purpose: a new [`HistoryAction`] variant cannot compile
/// until it is classified here, so none can be silently dropped.
pub fn reduce_history(ui: &mut UiState, action: HistoryAction) -> Vec<Effect> {
    match action {
        HistoryAction::Notify(message) => ui.notify(&message, false),
        // Clearing, confirmation, and deletion are owned by the history
        // coordinator and the service layer that holds the history store.
        HistoryAction::ClearActivity
        | HistoryAction::ClearHistory
        | HistoryAction::ClearHistoryConfirmed
        | HistoryAction::DeleteSelectedHistoryEntry
        | HistoryAction::DeleteHistoryEntry { .. } => {}
    }
    Vec::new()
}

/// Toggle the notification log overlay.
pub(crate) fn toggle_notification_log(ui: &mut UiState) {
    ui.show_notification_log = !ui.show_notification_log;
}

/// Switch the History view between its Recent and Top modes.
pub(crate) fn toggle_history_view_mode(ui: &mut UiState, domain: &DomainState) {
    ui.history_view_mode = match ui.history_view_mode {
        HistoryViewMode::Recent => HistoryViewMode::Top,
        HistoryViewMode::Top => HistoryViewMode::Recent,
    };
    ui.selected_index = 0;
    ui.reset_list(domain);
}

/// Dismiss the visible notification toast.
pub(crate) fn dismiss_notification(ui: &mut UiState) {
    ui.notification = None;
}

/// Scroll the now-playing description panel by signed rows.
pub(crate) fn scroll_now_playing(ui: &mut UiState, delta: i32) {
    let next = i32::from(ui.now_playing_scroll) + delta;
    ui.now_playing_scroll = next.max(0) as u16;
}

/// Toggle the Playing view's right pane between chapters and description.
pub(crate) fn toggle_now_playing_pane(ui: &mut UiState) {
    ui.now_playing_show_description = !ui.now_playing_show_description;
    ui.now_playing_scroll = 0;
}

/// Switch between info and queue focus in the ultra-wide Playing view.
pub(crate) fn cycle_playing_pane(ui: &mut UiState, domain: &DomainState) {
    if ui.view == View::NowPlaying
        && crate::render::layout::Breakpoint::from_width(ui.screen_area.width)
            == crate::render::layout::Breakpoint::UltraWide
    {
        ui.playing_pane = match ui.playing_pane {
            PlayingPane::Info => PlayingPane::Queue,
            PlayingPane::Queue => PlayingPane::Info,
        };
        if ui.playing_pane == PlayingPane::Queue {
            ui.selected_index = domain.queue.position.unwrap_or(0);
        }
        ui.reset_list(domain);
    }
}
