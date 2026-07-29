//! Notification and timer state.

use crate::app::state::AppState;

/// A transient user-facing notification (PRD section 16).
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub is_error: bool,
    pub created_at: chrono::DateTime<chrono::Local>,
    expires_at: std::time::Instant,
}

impl Notification {
    /// Build a notification with a deterministic creation instant.
    pub fn new_at(message: &str, is_error: bool, now: std::time::Instant) -> Self {
        let lifetime = if is_error {
            std::time::Duration::from_secs(8)
        } else {
            std::time::Duration::from_secs(4)
        };
        Self {
            message: message.to_string(),
            is_error,
            created_at: chrono::Local::now(),
            expires_at: now + lifetime,
        }
    }

    /// Whether this transient message has reached its deadline.
    pub fn is_expired_at(&self, now: std::time::Instant) -> bool {
        now >= self.expires_at
    }
}

impl crate::app::state::UiState {
    /// Record a transient notification and retain it in the bounded log.
    pub fn notify(&mut self, message: &str, is_error: bool) {
        let notification = Notification::new_at(message, is_error, std::time::Instant::now());
        self.notification_log.push_front(notification.clone());
        self.notification_log.truncate(50);
        self.notification = Some(notification);
    }
}

impl AppState {
    /// Record a transient notification and retain it in the bounded log.
    pub fn notify(&mut self, message: &str, is_error: bool) {
        self.ui.notify(message, is_error);
    }
}
