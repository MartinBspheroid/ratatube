use serde::{Deserialize, Serialize};

/// Availability of a track as far as the library knows (PRD 10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    #[default]
    Available,
    Unavailable,
    Private,
    Unknown,
}

/// A single playable item.
///
/// The canonical YouTube page URL is stored permanently; resolved stream
/// URLs are runtime-only state and are never persisted (PRD 10.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// YouTube video ID.
    pub id: String,
    pub title: String,
    /// Channel or artist name.
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
    #[serde(default)]
    pub availability: Availability,
}

impl Track {
    /// Construct a track with unknown availability and duration.
    pub fn new(id: impl Into<String>, title: impl Into<String>, artist: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            webpage_url: format!("https://www.youtube.com/watch?v={id}"),
            id,
            title: title.into(),
            artist: artist.into(),
            channel_id: None,
            channel_url: None,
            duration_seconds: None,
            thumbnail_url: None,
            availability: Availability::Unknown,
        }
    }
}
