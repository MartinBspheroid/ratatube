//! Remote playlist import flow and duplicate handling (PRD 10.8, 10.10).

use serde::{Deserialize, Serialize};

use crate::media::Track;
use crate::playlists::model::{Playlist, PlaylistSource};

/// Summary of a completed import for user review (PRD 10.8).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImportSummary {
    pub remote_title: String,
    pub remote_url: String,
    pub total_entries: usize,
    pub imported: usize,
    pub unavailable: usize,
    pub duplicates: usize,
    pub missing_metadata: usize,
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
    total_entries: usize,
    tracks: Vec<Track>,
    unavailable: usize,
) -> (Playlist, ImportSummary) {
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

    let missing_metadata = total_entries
        .saturating_sub(imported)
        .saturating_sub(unavailable)
        .saturating_sub(duplicates);

    let summary = ImportSummary {
        remote_title,
        remote_url,
        total_entries,
        imported,
        unavailable,
        duplicates,
        missing_metadata,
    };
    (playlist, summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str) -> Track {
        Track::new(id, id, "artist")
    }

    #[test]
    fn deduplicate_keeps_first_occurrence() {
        let tracks = vec![track("a"), track("b"), track("a"), track("c"), track("b")];
        let (kept, dupes) = deduplicate(tracks);
        let ids: Vec<&str> = kept.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
        assert_eq!(dupes, 2);
    }

    #[test]
    fn build_import_records_source_and_summary() {
        let (playlist, summary) = build_import(
            "My Mix".to_string(),
            "https://www.youtube.com/playlist?list=PLx".to_string(),
            Some("PLx".to_string()),
            10,
            vec![track("a"), track("a"), track("b")],
            5,
        );
        assert_eq!(summary.imported, 2);
        assert_eq!(summary.duplicates, 1);
        assert_eq!(summary.unavailable, 5);
        assert_eq!(summary.missing_metadata, 2);
        assert!(playlist.source.is_some());
        assert_eq!(playlist.tracks.len(), 2);
    }
}
