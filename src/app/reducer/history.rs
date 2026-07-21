//! History-family coordinator: activity is domain; view modes and
//! notifications are UI; destructive clears go through the confirm modal.

use crate::app::action::{Action, HistoryAction};
use crate::app::reducer::Effect;
use crate::app::state::AppState;

/// Reduce history and notification state transitions.
pub(super) fn reduce(state: &mut AppState, action: HistoryAction) -> Vec<Effect> {
    match action {
        HistoryAction::ClearActivity => {
            state.domain.activity.clear();
            vec![Effect::PersistSession]
        }
        HistoryAction::ClearHistory => {
            state.ui.confirm = Some(crate::app::state::ConfirmState {
                message: "Clear all playback history? (y/n)".to_string(),
                action: Box::new(Action::History(HistoryAction::ClearHistoryConfirmed)),
            });
            Vec::new()
        }
        // ClearHistoryConfirmed is applied by the service layer, which owns
        // the history store.
        HistoryAction::ClearHistoryConfirmed | HistoryAction::DeleteSelectedHistoryEntry => {
            Vec::new()
        }
        other => super::ui::presentation::reduce_history(&mut state.ui, &state.domain, other),
    }
}
