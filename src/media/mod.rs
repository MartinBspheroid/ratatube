//! Media discovery, metadata, and stream resolution via yt-dlp.

pub mod import;
pub mod resolver;
pub mod search;
pub mod yt_dlp;

use serde::{Deserialize, Serialize};

/// Extended metadata for a single video, fetched on demand for the
/// now-playing view and detail panels (PRD 10.1 metadata detail).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackDetails {
    pub description: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    /// yt-dlp `upload_date` as YYYYMMDD.
    pub upload_date: Option<String>,
    pub uploader: Option<String>,
    pub categories: Vec<String>,
}

impl TrackDetails {
    /// Upload date formatted as YYYY-MM-DD for display.
    pub fn formatted_upload_date(&self) -> Option<String> {
        let raw = self.upload_date.as_deref()?;
        if raw.len() == 8 {
            Some(format!("{}-{}-{}", &raw[0..4], &raw[4..6], &raw[6..8]))
        } else {
            Some(raw.to_string())
        }
    }
}

/// Format a large count compactly, e.g. 12_345_678 -> "12.3M".
pub fn format_count(count: u64) -> String {
    const MILLION: u64 = 1_000_000;
    const THOUSAND: u64 = 1_000;
    if count >= MILLION {
        format!("{:.1}M", count as f64 / MILLION as f64)
    } else if count >= THOUSAND {
        format!("{:.1}K", count as f64 / THOUSAND as f64)
    } else {
        count.to_string()
    }
}

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
            duration_seconds: None,
            thumbnail_url: None,
            availability: Availability::Unknown,
        }
    }
}
