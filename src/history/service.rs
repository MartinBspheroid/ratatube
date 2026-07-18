//! History persistence with configurable retention (PRD 10.11).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::history::model::HistoryEntry;
use crate::persistence::json_store;
use crate::persistence::migrations::HISTORY_SCHEMA_VERSION;

/// On-disk history document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDocument {
    pub schema_version: u32,
    /// Newest first.
    #[serde(default)]
    pub entries: Vec<HistoryEntry>,
}

/// Per-track aggregation of the history log ("Top" view).
#[derive(Debug, Clone)]
pub struct TrackStats {
    /// Most recent entry for the track (carries id, title, artist, URL).
    pub entry: HistoryEntry,
    pub plays: usize,
    pub total_listened_seconds: u64,
}

/// Append-only history store with a maximum entry count.
pub struct HistoryService {
    path: std::path::PathBuf,
    max_entries: usize,
    document: HistoryDocument,
}

impl HistoryService {
    /// Load history from disk, or start empty when missing.
    pub fn load(path: &Path, max_entries: usize) -> Result<Self> {
        let document = if path.exists() {
            json_store::read(path)?
        } else {
            HistoryDocument {
                schema_version: HISTORY_SCHEMA_VERSION,
                entries: Vec::new(),
            }
        };
        Ok(Self {
            path: path.to_path_buf(),
            max_entries,
            document,
        })
    }

    /// All entries, newest first.
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.document.entries
    }

    /// Record a play, trimming to `max_entries`.
    pub fn record(&mut self, entry: HistoryEntry) {
        self.document.entries.insert(0, entry);
        self.document.entries.truncate(self.max_entries);
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.document.entries.clear();
    }

    /// Remove one entry by index (newest-first order).
    pub fn remove(&mut self, index: usize) {
        if index < self.document.entries.len() {
            self.document.entries.remove(index);
        }
    }

    /// Aggregate the log per track: play counts and total listened time,
    /// most-played first (ties broken by most recent play).
    pub fn aggregate(&self) -> Vec<TrackStats> {
        let mut by_track: Vec<TrackStats> = Vec::new();
        let mut index: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for entry in &self.document.entries {
            match index.get(entry.track_id.as_str()) {
                Some(&i) => {
                    by_track[i].plays += 1;
                    by_track[i].total_listened_seconds += entry.listened_seconds;
                }
                None => {
                    index.insert(entry.track_id.as_str(), by_track.len());
                    by_track.push(TrackStats {
                        entry: entry.clone(),
                        plays: 1,
                        total_listened_seconds: entry.listened_seconds,
                    });
                }
            }
        }
        // Entries are newest-first, so each group's representative entry is
        // already its most recent play.
        by_track.sort_by(|a, b| {
            b.plays
                .cmp(&a.plays)
                .then(b.entry.played_at.cmp(&a.entry.played_at))
        });
        by_track
    }

    /// The most recent distinct tracks (newest first, deduplicated by
    /// track ID), for the Home dashboard's Recent section.
    pub fn recent_unique(&self, limit: usize) -> Vec<crate::media::Track> {
        let mut seen = std::collections::HashSet::new();
        self.document
            .entries
            .iter()
            .filter(|e| seen.insert(e.track_id.clone()))
            .take(limit)
            .map(HistoryEntry::to_track)
            .collect()
    }

    /// Persist history atomically.
    pub fn save(&self) -> Result<()> {
        json_store::atomic_write(&self.path, &self.document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::model::PlaybackOutcome;
    use crate::media::Track;

    #[test]
    fn retention_trims_oldest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history.json");
        let mut service = HistoryService::load(&path, 3).expect("load");
        for i in 0..5 {
            service.record(HistoryEntry::from_track(
                &Track::new(format!("t{i}"), "t", "a"),
                None,
                PlaybackOutcome::Completed,
                100,
            ));
        }
        assert_eq!(service.entries().len(), 3);
        assert_eq!(service.entries()[0].track_id, "t4");
    }

    #[test]
    fn aggregate_counts_plays_per_track() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut service =
            HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
        for _ in 0..3 {
            service.record(HistoryEntry::from_track(
                &Track::new("a", "Song A", "X"),
                None,
                PlaybackOutcome::Completed,
                100,
            ));
        }
        service.record(HistoryEntry::from_track(
            &Track::new("b", "Song B", "Y"),
            None,
            PlaybackOutcome::Skipped,
            10,
        ));
        let stats = service.aggregate();
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].entry.track_id, "a");
        assert_eq!(stats[0].plays, 3);
        assert_eq!(stats[0].total_listened_seconds, 300);
        assert_eq!(stats[1].plays, 1);
    }

    #[test]
    fn remove_deletes_one_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut service =
            HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
        for i in 0..3 {
            service.record(HistoryEntry::from_track(
                &Track::new(format!("t{i}"), "t", "a"),
                None,
                PlaybackOutcome::Completed,
                1,
            ));
        }
        service.remove(1);
        assert_eq!(service.entries().len(), 2);
        assert_eq!(service.entries()[0].track_id, "t2");
        assert_eq!(service.entries()[1].track_id, "t0");
        service.remove(99); // out of range is a no-op
        assert_eq!(service.entries().len(), 2);
    }

    #[test]
    fn save_and_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history.json");
        let mut service = HistoryService::load(&path, 500).expect("load");
        service.record(HistoryEntry::from_track(
            &Track::new("abc", "t", "a"),
            None,
            PlaybackOutcome::Skipped,
            12,
        ));
        service.save().expect("save");
        let reloaded = HistoryService::load(&path, 500).expect("reload");
        assert_eq!(reloaded.entries().len(), 1);
        assert_eq!(reloaded.entries()[0].outcome, PlaybackOutcome::Skipped);
    }
}
