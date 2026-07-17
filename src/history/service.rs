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
