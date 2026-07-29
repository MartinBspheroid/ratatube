//! History, activity, and notification actions.

/// An intent that changes persisted activity/history or visible notifications.
#[derive(Debug, Clone)]
pub enum HistoryAction {
    /// Clear the persisted Activity panel.
    ClearActivity,
    ClearHistory,
    /// Clear history after explicit confirmation.
    ClearHistoryConfirmed,
    /// Delete one history entry in Recent mode.
    DeleteSelectedHistoryEntry,
    /// Delete one history entry by store index, guarded by its track id
    /// (daemon clients, whose local view may lag the store).
    DeleteHistoryEntry {
        index: usize,
        expected_track_id: String,
    },
    Notify(String),
}
