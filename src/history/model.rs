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

/// Accumulates listened media time while excluding pauses and seek jumps.
#[derive(Debug, Default)]
pub struct ListeningAccumulator {
    playing: bool,
    last_position: Option<f64>,
    listened_seconds: f64,
}

impl ListeningAccumulator {
    /// Start a new track session.
    pub fn started(&mut self) {
        self.playing = true;
        self.last_position = None;
        self.listened_seconds = 0.0;
    }

    /// Update whether media is actively advancing.
    pub fn paused(&mut self, paused: bool) {
        self.playing = !paused;
        self.last_position = None;
    }

    /// Invalidate the position baseline before an explicit seek.
    pub fn seeking(&mut self) {
        self.last_position = None;
    }

    /// Consume one observed media position.
    pub fn position(&mut self, position: f64) {
        if !position.is_finite() || position < 0.0 {
            self.last_position = None;
            return;
        }
        if self.playing
            && let Some(previous) = self.last_position
        {
            let delta = position - previous;
            // Progress events normally arrive every 0.5s. Larger or
            // backwards jumps are seeks/stale events, not listened time.
            if delta > 0.0 && delta <= 3.0 {
                self.listened_seconds += delta;
            }
        }
        self.last_position = Some(position);
    }

    /// Finish the session and return whole media seconds listened.
    pub fn finish(&mut self) -> u64 {
        let listened = self.listened_seconds.floor() as u64;
        *self = Self::default();
        listened
    }
}

#[cfg(test)]
mod listening_tests {
    use super::ListeningAccumulator;

    #[test]
    fn counts_only_positive_playing_media_deltas() {
        let mut listening = ListeningAccumulator::default();
        listening.started();
        listening.position(10.0);
        listening.position(11.0);
        listening.paused(true);
        listening.position(12.0);
        listening.position(13.0);
        listening.paused(false);
        listening.position(13.0);
        listening.position(15.0);
        listening.seeking();
        listening.position(50.0);
        listening.position(51.5);
        listening.position(40.0);
        listening.position(41.0);

        assert_eq!(listening.finish(), 5);
    }

    #[test]
    fn large_position_jump_is_not_counted_as_listening() {
        let mut listening = ListeningAccumulator::default();
        listening.started();
        listening.position(0.0);
        listening.position(30.0);
        assert_eq!(listening.finish(), 0);
    }
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
            channel_id: None,
            channel_url: None,
            webpage_url: self.webpage_url.clone(),
            duration_seconds: None,
            thumbnail_url: None,
            availability: crate::media::Availability::Unknown,
        }
    }
}
