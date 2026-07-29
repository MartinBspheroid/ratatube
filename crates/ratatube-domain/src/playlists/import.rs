//! Remote playlist import flow and duplicate handling (PRD 10.8, 10.10).

mod json;

use serde::{Deserialize, Serialize};

use crate::media::Track;
use crate::playlists::model::{Playlist, PlaylistSource};

/// Parse a pasted versioned JSON document into validated local playlists.
/// No files are changed unless the entire document passes validation.
pub fn parse_pasted_json(input: &str) -> Result<Vec<Playlist>, String> {
    json::parse(input)
}

/// Summary of a completed import for user review (PRD 10.8).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImportSummary {
    pub remote_title: String,
    pub remote_url: String,
    pub total_entries: usize,
    pub imported: usize,
    pub deleted: usize,
    pub private: usize,
    pub unavailable: usize,
    pub duplicates: usize,
    pub missing_id: usize,
    pub missing_title: usize,
}

/// Progress of a running import, for the UI to display (PRD 17).
#[derive(Debug, Clone)]
pub enum ImportProgress {
    Validating,
    Fetching { url: String },
    Normalizing { done: usize, total: usize },
    ReadyForReview(Box<ImportSummary>, Box<Playlist>),
    Failed(String),
}

/// Deduplicate tracks by video ID, keeping the first occurrence (PRD 10.10).
/// Returns `(kept, duplicate_count)`.
pub fn deduplicate(tracks: Vec<Track>) -> (Vec<Track>, usize) {
    let mut seen = std::collections::HashSet::new();
    let mut kept = Vec::with_capacity(tracks.len());
    let mut duplicates = 0;
    for track in tracks {
        if seen.insert(track.id.clone()) {
            kept.push(track);
        } else {
            duplicates += 1;
        }
    }
    (kept, duplicates)
}

/// Build a local playlist from imported tracks plus a summary for review.
pub fn build_import(
    remote_title: String,
    remote_url: String,
    remote_id: Option<String>,
    tracks: Vec<Track>,
    rejections: crate::media::yt_dlp::ImportRejections,
) -> (Playlist, ImportSummary) {
    let total_entries = tracks.len() + rejections.total();
    let (kept, duplicates) = deduplicate(tracks);
    let imported = kept.len();

    let mut playlist = Playlist::new(&remote_title);
    playlist.source = Some(PlaylistSource {
        kind: "youtube-playlist".to_string(),
        url: remote_url.clone(),
        remote_id,
        last_synced_at: Some(chrono::Utc::now()),
    });
    playlist.tracks = kept
        .iter()
        .map(crate::playlists::model::PlaylistTrack::from)
        .collect();

    let summary = ImportSummary {
        remote_title,
        remote_url,
        total_entries,
        imported,
        deleted: rejections.deleted,
        private: rejections.private,
        unavailable: rejections.unavailable,
        duplicates,
        missing_id: rejections.missing_id,
        missing_title: rejections.missing_title,
    };
    (playlist, summary)
}

#[cfg(test)]
mod tests;
