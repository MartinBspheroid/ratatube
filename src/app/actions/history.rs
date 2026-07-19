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
    /// Toggle Recent and Top (aggregated) presentation.
    ToggleHistoryViewMode,
    Notify(String),
    DismissNotification,
    /// Toggle the notification log overlay (`!`).
    ToggleNotificationLog,
}
