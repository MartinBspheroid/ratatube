//! Playback history document model (PRD section 10.11).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::media::Track;

/// How a play session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaybackOutcome {
    Completed,
    Skipped,
    Failed,
    Stopped,
}

/// One entry in the play history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    /// YouTube video ID.
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub webpage_url: String,
    pub played_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlist_id: Option<String>,
    pub outcome: PlaybackOutcome,
    /// Approximate listened duration in seconds.
    pub listened_seconds: u64,
}

impl HistoryEntry {
    /// Build an entry from a track and outcome.
    pub fn from_track(
        track: &Track,
        playlist_id: Option<String>,
        outcome: PlaybackOutcome,
        listened_seconds: u64,
    ) -> Self {
        Self {
            track_id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            webpage_url: track.webpage_url.clone(),
            played_at: Utc::now(),
            playlist_id,
            outcome,
            listened_seconds,
        }
    }

    /// Convert back to a playable track.
    pub fn to_track(&self) -> Track {
        Track {
            id: self.track_id.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            webpage_url: self.webpage_url.clone(),
            duration_seconds: None,
            thumbnail_url: None,
            availability: crate::media::Availability::Unknown,
        }
    }
}
