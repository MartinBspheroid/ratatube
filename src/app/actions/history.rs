//! History, activity, and notification actions.

/// An intent that changes persisted activity/history or visible notifications.
#[derive(Debug, Clone)]
pub enum HistoryAction {
    ClearActivity,
    ClearHistory,
    ClearHistoryConfirmed,
    DeleteSelectedHistoryEntry,
    ToggleHistoryViewMode,
    Notify(String),
    DismissNotification,
    ToggleNotificationLog,
}
