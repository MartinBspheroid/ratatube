//! History, activity, and notification transitions.

use crate::app::action::{Action, HistoryAction};
use crate::app::reducer::Effect;
use crate::app::state::AppState;

pub(super) fn reduce(state: &mut AppState, action: HistoryAction) -> Vec<Effect> {
    match Action::History(action) {
        Action::History(HistoryAction::ClearActivity) => {
            state.activity.clear();
            return vec![Effect::PersistSession];
        }
        Action::History(HistoryAction::ToggleNotificationLog) => {
            state.show_notification_log = !state.show_notification_log;
        }
        Action::History(HistoryAction::ToggleHistoryViewMode) => {
            state.history_view_mode = match state.history_view_mode {
                crate::app::state::HistoryViewMode::Recent => {
                    crate::app::state::HistoryViewMode::Top
                }
                crate::app::state::HistoryViewMode::Top => {
                    crate::app::state::HistoryViewMode::Recent
                }
            };
            state.selected_index = 0;
            state.reset_list();
        }
        Action::History(HistoryAction::ClearHistory) => {
            state.confirm = Some(crate::app::state::ConfirmState {
                message: "Clear all playback history? (y/n)".to_string(),
                action: Box::new(Action::History(HistoryAction::ClearHistoryConfirmed)),
            });
        }

        // --- Modal UI ----------------------------------------------------
        // --- Notifications -------------------------------------------------
        Action::History(HistoryAction::Notify(message)) => state.notify(&message, false),
        Action::History(HistoryAction::DismissNotification) => state.notification = None,
        _ => {}
    }
    Vec::new()
}
