//! Remote playlist import flow and duplicate handling (PRD 10.8, 10.10).

use serde::{Deserialize, Serialize};

use crate::media::Track;
use crate::playlists::model::{Playlist, PlaylistSource};

const JSON_IMPORT_VERSION: u32 = 1;
const MAX_JSON_IMPORT_BYTES: usize = 1_048_576;
const MAX_JSON_PLAYLISTS: usize = 50;
const MAX_JSON_TRACKS: usize = 10_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonImportDocument {
    version: u32,
    playlists: Vec<JsonPlaylist>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonPlaylist {
    name: String,
    #[serde(default)]
    description: String,
    tracks: Vec<JsonTrack>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonTrack {
    title: String,
    channel: String,
    url: String,
}

/// Parse a pasted versioned JSON document into validated local playlists.
/// No files are changed unless the entire document passes validation.
pub fn parse_pasted_json(input: &str) -> Result<Vec<Playlist>, String> {
    if input.len() > MAX_JSON_IMPORT_BYTES {
        return Err("JSON import exceeds the 1 MiB limit".to_string());
    }
    let document: JsonImportDocument =
        serde_json::from_str(input).map_err(|error| format!("Invalid JSON: {error}"))?;
    if document.version != JSON_IMPORT_VERSION {
        return Err(format!(
            "Unsupported JSON import version {}; expected {JSON_IMPORT_VERSION}",
            document.version
        ));
    }
    if document.playlists.is_empty() {
        return Err("JSON import must contain at least one playlist".to_string());
    }
    if document.playlists.len() > MAX_JSON_PLAYLISTS {
        return Err(format!(
            "JSON import supports at most {MAX_JSON_PLAYLISTS} playlists"
        ));
    }
    let track_count = document
        .playlists
        .iter()
        .map(|playlist| playlist.tracks.len())
        .sum::<usize>();
    if track_count > MAX_JSON_TRACKS {
        return Err(format!(
            "JSON import supports at most {MAX_JSON_TRACKS} tracks"
        ));
    }

    document
        .playlists
        .into_iter()
        .map(|source| {
            let name = source.name.trim();
            if name.is_empty() {
                return Err("Every imported playlist needs a non-empty name".to_string());
            }
            if source.tracks.is_empty() {
                return Err(format!("Playlist \"{name}\" contains no tracks"));
            }
            let mut tracks = Vec::with_capacity(source.tracks.len());
            for item in source.tracks {
                let title = item.title.trim();
                let channel = item.channel.trim();
                let url = item.url.trim();
                let id = match crate::media::import::classify_input(url) {
                    crate::media::import::InputKind::Video(id) => id,
                    _ => {
                        return Err(format!(
                            "Playlist \"{name}\", track \"{title}\" needs a YouTube video URL"
                        ));
                    }
                };
                if title.is_empty() || channel.is_empty() {
                    return Err(format!(
                        "Playlist \"{name}\" has a track with an empty title or channel"
                    ));
                }
                let mut track = Track::new(id, title, channel);
                track.webpage_url = url.to_string();
                tracks.push(track);
            }
            let (tracks, _) = deduplicate(tracks);
            let mut playlist = Playlist::new(name);
            playlist.description = source.description.trim().to_string();
            playlist.tracks = tracks
                .iter()
                .map(crate::playlists::model::PlaylistTrack::from)
                .collect();
            Ok(playlist)
        })
        .collect()
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
            vec![track("a"), track("a"), track("b")],
            crate::media::yt_dlp::ImportRejections {
                malformed: 0,
                missing_id: 1,
                missing_title: 1,
                deleted: 2,
                private: 2,
                unavailable: 1,
            },
        );
        assert_eq!(summary.imported, 2);
        assert_eq!(summary.duplicates, 1);
        assert_eq!(summary.deleted, 2);
        assert_eq!(summary.private, 2);
        assert_eq!(summary.unavailable, 1);
        assert_eq!(summary.missing_id, 1);
        assert_eq!(summary.missing_title, 1);
        assert!(playlist.source.is_some());
        assert_eq!(playlist.tracks.len(), 2);
    }

    #[test]
    fn pasted_json_builds_multiple_local_playlists() {
        let json = r#"{
          "version": 1,
          "playlists": [
            {
              "name": "Neon Pressure",
              "description": "Jungle heat",
              "tracks": [
                {
                  "title": "Reset",
                  "channel": "Visages",
                  "url": "https://music.youtube.com/watch?v=sEltKu3XP6I"
                }
              ]
            },
            {
              "name": "Subterranean",
              "tracks": [
                {
                  "title": "Deepsoft",
                  "channel": "SCIENIDE 1995",
                  "url": "https://www.youtube.com/watch?v=OX838AIRC8M"
                }
              ]
            }
          ]
        }"#;

        let playlists = parse_pasted_json(json).expect("valid import");

        assert_eq!(playlists.len(), 2);
        assert_eq!(playlists[0].name, "Neon Pressure");
        assert_eq!(playlists[0].description, "Jungle heat");
        assert_eq!(playlists[0].tracks[0].id, "sEltKu3XP6I");
        assert_eq!(playlists[0].tracks[0].artist, "Visages");
        assert_eq!(playlists[1].tracks[0].id, "OX838AIRC8M");
        assert!(playlists.iter().all(|playlist| playlist.source.is_none()));
    }

    #[test]
    fn pasted_json_reports_the_invalid_track_location() {
        let json = r#"{
          "version": 1,
          "playlists": [{
            "name": "Broken",
            "tracks": [{"title": "No link", "channel": "Unknown", "url": ""}]
          }]
        }"#;

        let error = parse_pasted_json(json).expect_err("missing URL must fail");

        assert!(error.contains("Broken"), "{error}");
        assert!(error.contains("No link"), "{error}");
        assert!(error.contains("YouTube video URL"), "{error}");
    }

    #[test]
    fn checked_in_playlist_json_contains_all_specified_tracks() {
        let playlists = parse_pasted_json(include_str!("../../playlist.json"))
            .expect("checked-in playlist.json must remain importable");

        assert_eq!(playlists.len(), 7);
        assert_eq!(
            playlists
                .iter()
                .map(|playlist| playlist.tracks.len())
                .sum::<usize>(),
            154
        );
    }
}
