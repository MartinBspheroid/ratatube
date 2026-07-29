//! Last-session snapshot for instant resume on launch.
//!
//! A small document written throttled during playback and on shutdown so
//! the next launch can preload the last track at its saved position.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::history::activity::ActivityLog;
use crate::media::Track;
use crate::persistence::json_store;
use crate::persistence::migrations::migrate_in_place;
use crate::persistence::resume::ResumePoints;
use ratatube_domain::error::Result;

/// Current session document schema version.
pub const SESSION_SCHEMA_VERSION: u32 = 2;

/// Snapshot of what was playing when the app last saved state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDocument {
    pub schema_version: u32,
    /// The track that was playing, if any.
    pub track: Option<Track>,
    /// Playback position within the track, in seconds.
    #[serde(default)]
    pub position_seconds: f64,
    /// Volume at save time (0-100).
    #[serde(default)]
    pub volume: u8,
    pub saved_at: DateTime<Utc>,
    /// Persisted product activity, added in schema v2.
    #[serde(default)]
    pub activity: ActivityLog,
    /// Per-track meaningful playback positions, added in schema v2.
    #[serde(default)]
    pub resume_points: ResumePoints,
}

impl SessionDocument {
    /// Build a snapshot for the given playback moment.
    pub fn new(track: Option<Track>, position_seconds: f64, volume: u8) -> Self {
        Self {
            schema_version: SESSION_SCHEMA_VERSION,
            track,
            position_seconds,
            volume,
            saved_at: Utc::now(),
            activity: ActivityLog::default(),
            resume_points: ResumePoints::default(),
        }
    }
}

/// Load the last session, or `None` when missing or unreadable. A corrupt
/// session file must never block startup, so read errors degrade to `None`
/// (json_store already preserves a `.bak`).
pub fn load(path: &Path) -> Option<SessionDocument> {
    if !path.exists() {
        return None;
    }
    if let Err(err) = migrate_in_place(path, SESSION_SCHEMA_VERSION) {
        tracing::warn!(?err, "session schema migration failed");
        return None;
    }
    match json_store::read::<SessionDocument>(path) {
        Ok(doc) if doc.schema_version <= SESSION_SCHEMA_VERSION => Some(doc),
        Ok(_) => {
            tracing::warn!(?path, "session document from a newer schema; ignoring");
            None
        }
        Err(err) => {
            tracing::warn!(?err, "session restore failed");
            None
        }
    }
}

/// Persist the session snapshot atomically.
pub fn save(path: &Path, document: &SessionDocument) -> Result<()> {
    json_store::atomic_write(path, document)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn session_roundtrips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("session.json");
        let mut doc = SessionDocument::new(Some(Track::new("id1", "Title", "Artist")), 123.5, 60);
        doc.activity
            .push(crate::history::activity::ActivityEvent::new(
                crate::history::activity::ActivityKind::Queued,
                "Title",
                "Artist",
            ));
        assert!(doc.resume_points.record("id1", 123.5, 300.0, Utc::now()));
        save(&path, &doc).expect("save");
        let loaded = load(&path).expect("load");
        assert_eq!(loaded.track.map(|t| t.id), Some("id1".to_string()));
        assert_eq!(loaded.position_seconds, 123.5);
        assert_eq!(loaded.volume, 60);
        assert_eq!(loaded.activity.len(), 1);
        assert_eq!(loaded.resume_points.len(), 1);
    }

    #[test]
    fn missing_file_is_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(load(&dir.path().join("session.json")).is_none());
    }

    #[test]
    fn load_migrates_pre_versioned_session() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("session.json");
        let doc = SessionDocument::new(None, 0.0, 50);
        let mut raw = serde_json::to_value(&doc).expect("serialize");
        raw.as_object_mut().expect("object").remove("schemaVersion");
        fs::write(&path, serde_json::to_vec_pretty(&raw).expect("json")).expect("write");

        assert!(load(&path).is_some());
        let migrated: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("parse");
        assert_eq!(migrated["schemaVersion"], SESSION_SCHEMA_VERSION);
    }

    #[test]
    fn load_ignores_future_session_without_rewriting() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("session.json");
        let doc = SessionDocument::new(None, 0.0, 50);
        let mut raw = serde_json::to_value(&doc).expect("serialize");
        raw["schemaVersion"] = serde_json::json!(99);
        let original = serde_json::to_vec_pretty(&raw).expect("json");
        fs::write(&path, &original).expect("write");

        assert!(load(&path).is_none());
        assert_eq!(fs::read(&path).expect("unchanged"), original);
    }

    #[test]
    fn schema_one_session_loads_with_default_v2_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("session.json");
        let raw = serde_json::json!({
            "schemaVersion": 1,
            "track": null,
            "positionSeconds": 0.0,
            "volume": 50,
            "savedAt": Utc::now(),
        });
        fs::write(&path, serde_json::to_vec_pretty(&raw).expect("json")).expect("write");

        let loaded = load(&path).expect("forward-compatible load");
        assert!(loaded.activity.is_empty());
        assert_eq!(loaded.resume_points.len(), 0);
        assert_eq!(loaded.schema_version, SESSION_SCHEMA_VERSION);
    }

    #[test]
    fn legacy_shape_ignores_new_fields_during_deserialization() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct LegacySession {
            schema_version: u32,
            track: Option<Track>,
            position_seconds: f64,
            volume: u8,
            saved_at: DateTime<Utc>,
        }

        let modern = SessionDocument::new(None, 12.0, 55);
        let raw = serde_json::to_string(&modern).expect("serialize modern");
        let legacy: LegacySession = serde_json::from_str(&raw).expect("unknown fields ignored");
        assert_eq!(legacy.schema_version, SESSION_SCHEMA_VERSION);
        assert!(legacy.track.is_none());
        assert_eq!(legacy.position_seconds, 12.0);
        assert_eq!(legacy.volume, 55);
        assert!(legacy.saved_at <= Utc::now());
    }
}
