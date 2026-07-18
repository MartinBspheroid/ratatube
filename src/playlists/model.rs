//! Playlist document model (PRD section 11.2).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::media::{Availability, Track};
use crate::persistence::migrations::PLAYLIST_SCHEMA_VERSION;

/// Origin of an imported playlist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSource {
    /// e.g. "youtube-playlist".
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub remote_id: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
}

/// A track as stored inside a playlist document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTrack {
    /// YouTube video ID.
    pub id: String,
    pub title: String,
    pub artist: String,
    /// Stable YouTube channel identifier, when available.
    #[serde(default)]
    pub channel_id: Option<String>,
    /// Canonical YouTube channel URL, when available.
    #[serde(default)]
    pub channel_url: Option<String>,
    pub webpage_url: String,
    pub duration_seconds: Option<u64>,
    pub thumbnail_url: Option<String>,
    pub added_at: DateTime<Utc>,
    #[serde(default)]
    pub availability: Availability,
}

impl From<&Track> for PlaylistTrack {
    fn from(track: &Track) -> Self {
        Self {
            id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            channel_id: track.channel_id.clone(),
            channel_url: track.channel_url.clone(),
            webpage_url: track.webpage_url.clone(),
            duration_seconds: track.duration_seconds,
            thumbnail_url: track.thumbnail_url.clone(),
            added_at: Utc::now(),
            availability: track.availability,
        }
    }
}

impl From<&PlaylistTrack> for Track {
    fn from(track: &PlaylistTrack) -> Self {
        Self {
            id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            channel_id: track.channel_id.clone(),
            channel_url: track.channel_url.clone(),
            webpage_url: track.webpage_url.clone(),
            duration_seconds: track.duration_seconds,
            thumbnail_url: track.thumbnail_url.clone(),
            availability: track.availability,
        }
    }
}

/// A local playlist document, stored one file per playlist (PRD 11.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub schema_version: u32,
    /// Stable locally generated ID, independent of the title (PRD 10.7).
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PlaylistSource>,
    #[serde(default)]
    pub tracks: Vec<PlaylistTrack>,
}

impl Playlist {
    /// Create a new empty local playlist.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            schema_version: PLAYLIST_SCHEMA_VERSION,
            id: Ulid::generate().to_string(),
            name: name.into(),
            description: String::new(),
            created_at: now,
            updated_at: now,
            source: None,
            tracks: Vec::new(),
        }
    }

    /// Convert the full track list to runtime tracks.
    pub fn to_tracks(&self) -> Vec<Track> {
        self.tracks.iter().map(Track::from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_roundtrips_without_stream_urls() {
        let mut playlist = Playlist::new("Late Night Coding");
        let mut track = Track::new("dQw4w9WgXcQ", "Track title", "Channel");
        track.channel_id = Some("UC123".to_string());
        track.channel_url = Some("https://www.youtube.com/channel/UC123".to_string());
        playlist.tracks.push(PlaylistTrack::from(&track));
        let json = serde_json::to_string_pretty(&playlist).expect("serialize");
        assert!(!json.contains("googlevideo"), "no stream URLs persisted");
        let parsed: Playlist = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, PLAYLIST_SCHEMA_VERSION);
        assert_eq!(parsed.tracks.len(), 1);
        let roundtripped = Track::from(&parsed.tracks[0]);
        assert_eq!(roundtripped.channel_id.as_deref(), Some("UC123"));
        assert_eq!(
            roundtripped.channel_url.as_deref(),
            Some("https://www.youtube.com/channel/UC123")
        );
    }

    #[test]
    fn legacy_track_defaults_channel_identity() {
        let track: Track = serde_json::from_str(
            r#"{"id":"v","title":"Video","artist":"Channel","webpageUrl":"https://www.youtube.com/watch?v=v","durationSeconds":null,"thumbnailUrl":null,"availability":"unknown"}"#,
        )
        .expect("legacy track");
        assert_eq!(track.channel_id, None);
        assert_eq!(track.channel_url, None);
    }
}
