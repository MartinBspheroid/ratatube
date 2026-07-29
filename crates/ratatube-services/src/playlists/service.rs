//! Playlist CRUD over the one-file-per-playlist directory (PRD 11.1).

use std::path::PathBuf;

use crate::persistence::json_store;
use crate::persistence::migrations::{PLAYLIST_SCHEMA_VERSION, migrate_in_place};
use crate::playlists::model::Playlist;
use ratatube_domain::error::{AppError, Result};

/// Filesystem-backed playlist repository.
#[derive(Clone)]
pub struct PlaylistService {
    dir: PathBuf,
}

impl PlaylistService {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Create the storage directory if needed.
    pub fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        Ok(())
    }

    fn file_for(&self, id: &str) -> Result<PathBuf> {
        // Guard against path traversal in IDs (PRD section 19).
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::Storage {
                path: self.dir.join(id),
                message: format!("invalid playlist id {id:?}"),
            });
        }
        Ok(self.dir.join(format!("{id}.json")))
    }

    /// List all playlists with the most recently updated first.
    pub fn list(&self) -> Result<Vec<Playlist>> {
        let mut playlists = Vec::new();
        if !self.dir.exists() {
            return Ok(playlists);
        }
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                match migrate_in_place(&path, PLAYLIST_SCHEMA_VERSION)
                    .and_then(|_| json_store::read::<Playlist>(&path))
                {
                    Ok(playlist) => playlists.push(playlist),
                    // A single malformed playlist must not break the listing.
                    Err(err) => tracing::warn!(?err, ?path, "skipping unreadable playlist"),
                }
            }
        }
        playlists.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(playlists)
    }

    /// Load one playlist by ID.
    pub fn get(&self, id: &str) -> Result<Playlist> {
        let path = self.file_for(id)?;
        migrate_in_place(&path, PLAYLIST_SCHEMA_VERSION)?;
        json_store::read(&path)
    }

    /// Persist a playlist atomically.
    pub fn save(&self, playlist: &Playlist) -> Result<()> {
        self.ensure_dir()?;
        json_store::atomic_write(&self.file_for(&playlist.id)?, playlist)
    }

    /// Delete a playlist file. Callers must confirm with the user first
    /// (PRD 10.7); deletion is local-only and never touches YouTube.
    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.file_for(id)?;
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Duplicate a playlist under a new ID with `<name> (copy)`.
    pub fn duplicate(&self, id: &str) -> Result<Playlist> {
        let original = self.get(id)?;
        let mut copy = original.clone();
        copy.id = ulid::Ulid::generate().to_string();
        copy.name = format!("{} (copy)", original.name);
        copy.created_at = chrono::Utc::now();
        copy.updated_at = copy.created_at;
        self.save(&copy)?;
        Ok(copy)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn save_list_delete_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = PlaylistService::new(dir.path().join("playlists"));
        let playlist = Playlist::new("Test");
        let id = playlist.id.clone();
        service.save(&playlist).expect("save");
        assert_eq!(service.list().expect("list").len(), 1);
        assert_eq!(service.get(&id).expect("get").name, "Test");
        service.delete(&id).expect("delete");
        assert!(service.list().expect("list").is_empty());
    }

    #[test]
    fn list_orders_playlists_by_most_recent_update() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = PlaylistService::new(dir.path().join("playlists"));
        let mut older = Playlist::new("A older");
        older.updated_at = chrono::Utc::now() - chrono::Duration::days(1);
        let newer = Playlist::new("Z newer");
        service.save(&older).expect("save older");
        service.save(&newer).expect("save newer");

        let listed = service.list().expect("list");
        assert_eq!(listed[0].id, newer.id);
        assert_eq!(listed[1].id, older.id);
    }

    #[test]
    fn rejects_path_traversal_ids() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = PlaylistService::new(dir.path().to_path_buf());
        assert!(service.get("../../etc/passwd").is_err());
    }

    #[test]
    fn duplicate_creates_new_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = PlaylistService::new(dir.path().to_path_buf());
        let playlist = Playlist::new("Original");
        service.save(&playlist).expect("save");
        let copy = service.duplicate(&playlist.id).expect("duplicate");
        assert_ne!(copy.id, playlist.id);
        assert_eq!(copy.name, "Original (copy)");
    }

    #[test]
    fn get_migrates_pre_versioned_playlist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = PlaylistService::new(dir.path().to_path_buf());
        let playlist = Playlist::new("Legacy");
        let path = dir.path().join(format!("{}.json", playlist.id));
        let mut raw = serde_json::to_value(&playlist).expect("serialize");
        raw.as_object_mut().expect("object").remove("schemaVersion");
        fs::write(&path, serde_json::to_vec_pretty(&raw).expect("json")).expect("write");

        let loaded = service.get(&playlist.id).expect("load");

        assert_eq!(loaded.name, "Legacy");
        let migrated: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("parse");
        assert_eq!(migrated["schemaVersion"], PLAYLIST_SCHEMA_VERSION);
    }

    #[test]
    fn get_rejects_future_playlist_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = PlaylistService::new(dir.path().to_path_buf());
        let playlist = Playlist::new("Future");
        let path = dir.path().join(format!("{}.json", playlist.id));
        let mut raw = serde_json::to_value(&playlist).expect("serialize");
        raw["schemaVersion"] = serde_json::json!(99);
        fs::write(&path, serde_json::to_vec_pretty(&raw).expect("json")).expect("write");

        let result = service.get(&playlist.id);

        assert!(matches!(
            result,
            Err(AppError::UnsupportedSchema { found: 99, .. })
        ));
    }
}
