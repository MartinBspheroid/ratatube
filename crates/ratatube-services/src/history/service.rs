//! History persistence with configurable retention (PRD 10.11).
//!
//! The log itself is domain state; this type owns only its path and its
//! atomic read/write. It derefs to the log so callers keep one vocabulary.

use std::ops::{Deref, DerefMut};
use std::path::Path;

use ratatube_domain::error::Result;
use ratatube_domain::history::{HistoryDocument, HistoryLog};

use crate::persistence::json_store;
use crate::persistence::migrations::{HISTORY_SCHEMA_VERSION, migrate_in_place};

/// Persistent playback history: a domain log plus the file backing it.
#[derive(Clone)]
pub struct HistoryService {
    path: std::path::PathBuf,
    log: HistoryLog,
}

impl HistoryService {
    /// Load history from disk, or start empty when missing.
    pub fn load(path: &Path, max_entries: usize) -> Result<Self> {
        let document = if path.exists() {
            migrate_in_place(path, HISTORY_SCHEMA_VERSION)?;
            json_store::read(path)?
        } else {
            HistoryDocument {
                schema_version: HISTORY_SCHEMA_VERSION,
                entries: Vec::new(),
            }
        };
        Ok(Self {
            path: path.to_path_buf(),
            log: HistoryLog::new(document, max_entries),
        })
    }

    /// Persist history atomically.
    pub fn save(&self) -> Result<()> {
        json_store::atomic_write(&self.path, self.log.document())
    }
}

impl Deref for HistoryService {
    type Target = HistoryLog;

    fn deref(&self) -> &Self::Target {
        &self.log
    }
}

impl DerefMut for HistoryService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.log
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::history::model::{HistoryEntry, PlaybackOutcome};
    use crate::media::Track;
    use ratatube_domain::error::AppError;

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
        let mut service = HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
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
        assert_eq!(stats[0].completed_plays, 3);
        assert_eq!(stats[0].attempts, 3);
        assert_eq!(stats[0].total_listened_seconds, 300);
        assert_eq!(stats[1].completed_plays, 0);
        assert_eq!(stats[1].attempts, 1);
    }

    #[test]
    fn recent_unique_indices_keep_only_the_newest_event_per_track() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut service = HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
        service.record(HistoryEntry::from_track(
            &Track::new("a", "Old A", "Channel"),
            None,
            PlaybackOutcome::Stopped,
            10,
        ));
        service.record(HistoryEntry::from_track(
            &Track::new("b", "Only B", "Channel"),
            None,
            PlaybackOutcome::Completed,
            20,
        ));
        service.record(HistoryEntry::from_track(
            &Track::new("a", "Newest A", "Channel"),
            None,
            PlaybackOutcome::Completed,
            30,
        ));

        assert_eq!(service.recent_unique_indices(), vec![0, 1]);
        assert_eq!(
            service.entries()[service.recent_unique_indices()[0]].title,
            "Newest A"
        );
    }

    #[test]
    fn aggregate_is_cached_and_invalidated_by_mutation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut service = HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
        service.record(HistoryEntry::from_track(
            &Track::new("a", "A", "Artist"),
            None,
            PlaybackOutcome::Completed,
            10,
        ));
        let first = service.aggregate().as_ptr();
        assert_eq!(first, service.aggregate().as_ptr());
        service.record(HistoryEntry::from_track(
            &Track::new("b", "B", "Artist"),
            None,
            PlaybackOutcome::Completed,
            10,
        ));
        assert_eq!(service.aggregate().len(), 2);
    }

    #[test]
    fn remove_deletes_one_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut service = HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
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

    #[test]
    fn load_migrates_pre_versioned_history() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history.json");
        fs::write(&path, r#"{"entries":[]}"#).expect("write");

        let service = HistoryService::load(&path, 500).expect("load");

        assert!(service.entries().is_empty());
        let migrated: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("parse");
        assert_eq!(migrated["schemaVersion"], HISTORY_SCHEMA_VERSION);
    }

    #[test]
    fn load_rejects_future_history_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history.json");
        fs::write(&path, r#"{"schemaVersion":99,"entries":[]}"#).expect("write");

        let result = HistoryService::load(&path, 500);

        assert!(matches!(
            result,
            Err(AppError::UnsupportedSchema { found: 99, .. })
        ));
    }
}
