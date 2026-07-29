//! Bounded per-track resume positions stored with the session document.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const RESUME_CAPACITY: usize = 50;

/// Last meaningful playback position for one video.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumePoint {
    pub video_id: String,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub updated_at: DateTime<Utc>,
}

/// Most-recently-updated map encoded as an ordered list for stable JSON.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResumePoints(VecDeque<ResumePoint>);

impl ResumePoints {
    /// Store a meaningful point, excluding starts and effectively completed tracks.
    pub fn record(
        &mut self,
        video_id: impl Into<String>,
        position_seconds: f64,
        duration_seconds: f64,
        updated_at: DateTime<Utc>,
    ) -> bool {
        if !position_seconds.is_finite()
            || !duration_seconds.is_finite()
            || duration_seconds <= 0.0
            || position_seconds <= 10.0
            || position_seconds >= duration_seconds * 0.95
        {
            return false;
        }
        let video_id = video_id.into();
        self.0.retain(|point| point.video_id != video_id);
        self.0.push_front(ResumePoint {
            video_id,
            position_seconds,
            duration_seconds,
            updated_at,
        });
        self.0.truncate(RESUME_CAPACITY);
        true
    }

    /// Read points from most to least recently updated.
    pub fn entries(&self) -> &VecDeque<ResumePoint> {
        &self.0
    }

    /// Number of retained resume points.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no resume points are retained.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_start_and_completed_positions() {
        let mut points = ResumePoints::default();
        assert!(!points.record("start", 10.0, 100.0, Utc::now()));
        assert!(!points.record("complete", 95.0, 100.0, Utc::now()));
        assert!(points.entries().is_empty());
    }

    #[test]
    fn updates_lru_order_and_caps_at_fifty() {
        let mut points = ResumePoints::default();
        for index in 0..55 {
            assert!(points.record(index.to_string(), 20.0, 100.0, Utc::now()));
        }
        assert_eq!(points.len(), 50);
        assert!(points.record("10", 30.0, 100.0, Utc::now()));
        assert_eq!(
            points
                .entries()
                .front()
                .map(|point| point.video_id.as_str()),
            Some("10")
        );
    }
}
