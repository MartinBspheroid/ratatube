//! Presentation-mode toggles and notification transitions.

use crate::app::action::HistoryAction;
use crate::app::reducer::Effect;
use crate::app::state::{DomainState, HistoryViewMode, UiState};

/// Reduce UI-only history-family transitions (view modes, notifications).
pub(in crate::app::reducer) fn reduce_history(
    ui: &mut UiState,
    domain: &DomainState,
    action: HistoryAction,
) -> Vec<Effect> {
    match action {
        HistoryAction::ToggleNotificationLog => {
            ui.show_notification_log = !ui.show_notification_log;
        }
        HistoryAction::ToggleHistoryViewMode => {
            ui.history_view_mode = match ui.history_view_mode {
                HistoryViewMode::Recent => HistoryViewMode::Top,
                HistoryViewMode::Top => HistoryViewMode::Recent,
            };
            ui.selected_index = 0;
            ui.reset_list(domain);
        }
        HistoryAction::Notify(message) => ui.notify(&message, false),
        HistoryAction::DismissNotification => ui.notification = None,
        _ => {}
    }
    Vec::new()
}
