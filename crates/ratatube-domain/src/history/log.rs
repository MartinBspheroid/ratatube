//! In-memory playback history with retention and per-track aggregation.
//!
//! Reading and writing the document is a service concern; everything the UI
//! renders and the reducers query lives here.

use serde::{Deserialize, Serialize};

use crate::history::model::{HistoryEntry, PlaybackOutcome};
use crate::media::Track;

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
    /// Every recorded session regardless of outcome.
    pub attempts: usize,
    /// Sessions that reached natural completion.
    pub completed_plays: usize,
    pub total_listened_seconds: u64,
}

/// Append-only history log with a maximum entry count.
#[derive(Clone)]
pub struct HistoryLog {
    max_entries: usize,
    document: HistoryDocument,
    aggregate_cache: std::cell::OnceCell<Vec<TrackStats>>,
}

impl HistoryLog {
    /// Wrap a loaded document with its retention limit.
    pub fn new(document: HistoryDocument, max_entries: usize) -> Self {
        Self {
            max_entries,
            document,
            aggregate_cache: std::cell::OnceCell::new(),
        }
    }

    /// The document to persist.
    pub fn document(&self) -> &HistoryDocument {
        &self.document
    }

    /// All entries, newest first.
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.document.entries
    }

    /// Record a play, trimming to the retention limit.
    pub fn record(&mut self, entry: HistoryEntry) {
        self.aggregate_cache.take();
        self.document.entries.insert(0, entry);
        self.document.entries.truncate(self.max_entries);
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.aggregate_cache.take();
        self.document.entries.clear();
    }

    /// Remove one entry by index (newest-first order).
    pub fn remove(&mut self, index: usize) {
        if index < self.document.entries.len() {
            self.aggregate_cache.take();
            self.document.entries.remove(index);
        }
    }

    /// Aggregate attempts, completed plays, and listened media time per track.
    pub fn aggregate(&self) -> &[TrackStats] {
        self.aggregate_cache.get_or_init(|| {
            let mut by_track: Vec<TrackStats> = Vec::new();
            let mut index: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for entry in &self.document.entries {
                match index.get(entry.track_id.as_str()) {
                    Some(&i) => {
                        by_track[i].attempts += 1;
                        if entry.outcome == PlaybackOutcome::Completed {
                            by_track[i].completed_plays += 1;
                        }
                        by_track[i].total_listened_seconds += entry.listened_seconds;
                    }
                    None => {
                        index.insert(entry.track_id.as_str(), by_track.len());
                        by_track.push(TrackStats {
                            entry: entry.clone(),
                            attempts: 1,
                            completed_plays: usize::from(
                                entry.outcome == PlaybackOutcome::Completed,
                            ),
                            total_listened_seconds: entry.listened_seconds,
                        });
                    }
                }
            }
            // Entries are newest-first, so each representative is the latest.
            by_track.sort_by(|a, b| {
                b.completed_plays
                    .cmp(&a.completed_plays)
                    .then(b.attempts.cmp(&a.attempts))
                    .then(b.entry.played_at.cmp(&a.entry.played_at))
            });
            by_track
        })
    }

    /// The most recent distinct tracks (newest first, deduplicated by
    /// track ID), for the Home dashboard's Recent section.
    pub fn recent_unique(&self, limit: usize) -> Vec<Track> {
        self.recent_unique_indices()
            .into_iter()
            .take(limit)
            .map(|index| self.document.entries[index].to_track())
            .collect()
    }

    /// Underlying newest-first indices for one representative per track ID.
    pub fn recent_unique_indices(&self) -> Vec<usize> {
        let mut seen = std::collections::HashSet::new();
        self.document
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| seen.insert(entry.track_id.as_str()).then_some(index))
            .collect()
    }
}
