//! History actions requiring the persistence-backed history service.

use crate::app::App;
use crate::app::action::HistoryAction;
use crate::app::state::HistoryViewMode;

impl App {
    /// Apply history mutations that require the persistence-backed history service.
    pub(super) fn handle_history_service(&mut self, action: HistoryAction) {
        match action {
            HistoryAction::DeleteSelectedHistoryEntry => {
                if self.state.history_view_mode != HistoryViewMode::Recent {
                    self.state
                        .notify("Switch to recent (g) to delete entries", false);
                    return;
                }
                let index = self.state.resolve_index(self.state.selected_index);
                if let Some(history) = self.history.as_mut() {
                    history.remove(index);
                    self.state.history_len = history.recent_unique_indices().len();
                    self.state.clamp_selection();
                }
                self.list_revision = self.list_revision.wrapping_add(1);
                self.persist_history();
            }
            HistoryAction::ClearHistoryConfirmed => {
                if let Some(history) = self.history.as_mut() {
                    history.clear();
                }
                self.persist_history();
                self.state.notify("History cleared", false);
            }
            _ => {}
        }
    }
}
