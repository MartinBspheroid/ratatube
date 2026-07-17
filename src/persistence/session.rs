//! Last-session snapshot for instant resume on launch.
//!
//! A small document written throttled during playback and on shutdown so
//! the next launch can preload the last track at its saved position.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::media::Track;
use crate::persistence::json_store;

/// Current session document schema version.
pub const SESSION_SCHEMA_VERSION: u32 = 1;

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
    use super::*;

    #[test]
    fn session_roundtrips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("session.json");
        let doc = SessionDocument::new(Some(Track::new("id1", "Title", "Artist")), 123.5, 60);
        save(&path, &doc).expect("save");
        let loaded = load(&path).expect("load");
        assert_eq!(loaded.track.map(|t| t.id), Some("id1".to_string()));
        assert_eq!(loaded.position_seconds, 123.5);
        assert_eq!(loaded.volume, 60);
    }

    #[test]
    fn missing_file_is_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(load(&dir.path().join("session.json")).is_none());
    }
}
