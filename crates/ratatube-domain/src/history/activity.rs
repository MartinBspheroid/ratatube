//! Persisted, bounded user activity for dashboard context.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const ACTIVITY_CAPACITY: usize = 100;

/// Activity kinds the application can emit today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivityKind {
    Played,
    Queued,
    AddedToPlaylist,
    PlaylistImported,
}

/// One truthful user-visible activity entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub kind: ActivityKind,
    pub title: String,
    pub detail: String,
    pub at: DateTime<Utc>,
}

impl ActivityEvent {
    /// Build an event at the current wall-clock instant.
    pub fn new(kind: ActivityKind, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            detail: detail.into(),
            at: Utc::now(),
        }
    }
}

/// Most-recent-first bounded activity log.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActivityLog(VecDeque<ActivityEvent>);

impl ActivityLog {
    /// Add an event and retain at most the newest 100 entries.
    pub fn push(&mut self, event: ActivityEvent) {
        self.0.push_front(event);
        self.0.truncate(ACTIVITY_CAPACITY);
    }

    /// Remove every activity event.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Read events newest first.
    pub fn entries(&self) -> &VecDeque<ActivityEvent> {
        &self.0
    }

    /// Number of retained entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no events are retained.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Format an event age without consulting global time inside the renderer.
pub fn relative_time(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = (now - at).num_seconds().max(0);
    match seconds {
        0..60 => "now".to_string(),
        60..3600 => format!("{}m", seconds / 60),
        3600..86_400 => format!("{}h", seconds / 3600),
        _ => format!("{}d", seconds / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn log_keeps_only_newest_hundred_events() {
        let mut log = ActivityLog::default();
        for index in 0..105 {
            log.push(ActivityEvent::new(
                ActivityKind::Queued,
                index.to_string(),
                "",
            ));
        }
        assert_eq!(log.len(), 100);
        assert_eq!(
            log.entries().front().map(|event| event.title.as_str()),
            Some("104")
        );
        assert_eq!(
            log.entries().back().map(|event| event.title.as_str()),
            Some("5")
        );
    }

    #[test]
    fn relative_time_boundaries_are_deterministic() {
        let now = Utc::now();
        assert_eq!(relative_time(now - Duration::seconds(59), now), "now");
        assert_eq!(relative_time(now - Duration::seconds(60), now), "1m");
        assert_eq!(relative_time(now - Duration::hours(2), now), "2h");
        assert_eq!(relative_time(now - Duration::days(3), now), "3d");
    }
}
