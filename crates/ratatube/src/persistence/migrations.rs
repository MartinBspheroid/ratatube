//! Schema versioning and explicit migrations (PRD section 11.5).
//!
//! Every persisted document carries `schemaVersion`. Migrations run in
//! sequence, always create a backup first, and never discard unknown data
//! silently.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::{AppError, Result};

/// Highest schema version this build understands per document kind.
pub use ratatube_domain::schema::{
    HISTORY_SCHEMA_VERSION, PLAYLIST_SCHEMA_VERSION, QUEUE_SCHEMA_VERSION,
};

/// Read the declared schema version of a raw JSON document.
pub fn declared_version(raw: &Value) -> u32 {
    raw.get("schemaVersion")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}

/// Migrate the document at `path` to `target` version if needed.
///
/// Creates a `<path>.pre-migration.bak` backup before any modification.
/// Returns an error when the document is newer than `target`.
pub fn migrate_in_place(path: &Path, target: u32) -> Result<bool> {
    let raw_text = fs::read_to_string(path)?;
    let raw: Value = serde_json::from_str(&raw_text)?;
    let version = declared_version(&raw);
    if version > target {
        return Err(AppError::UnsupportedSchema {
            path: path.to_path_buf(),
            found: version,
            supported: target,
        });
    }
    if version == target {
        return Ok(false);
    }

    let backup = path.with_extension("pre-migration.bak");
    fs::copy(path, &backup)?;

    // Version zero is the pre-versioned form of the current document shape.
    // Future structural migrations must be added here one version at a time.
    let mut migrated = raw;
    let Some(object) = migrated.as_object_mut() else {
        return Err(AppError::Storage {
            path: path.to_path_buf(),
            message: "versioned document must be a JSON object".to_string(),
        });
    };
    object.insert("schemaVersion".to_string(), Value::from(target));
    crate::persistence::json_store::atomic_write(path, &migrated)?;
    tracing::info!(?path, from = version, to = target, "migrated document");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_schema_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        fs::write(&path, r#"{"schemaVersion": 99}"#).expect("write");
        let result = migrate_in_place(&path, 1);
        assert!(matches!(
            result,
            Err(AppError::UnsupportedSchema { found: 99, .. })
        ));
    }

    #[test]
    fn current_schema_is_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        fs::write(&path, r#"{"schemaVersion": 1}"#).expect("write");
        assert!(!migrate_in_place(&path, 1).expect("migrate"));
    }

    #[test]
    fn version_zero_is_backed_up_and_updated_to_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        fs::write(&path, r#"{"entries":[]}"#).expect("write");

        assert!(migrate_in_place(&path, 1).expect("migrate"));

        let migrated: Value = serde_json::from_slice(&fs::read(&path).expect("read migrated"))
            .expect("parse migrated");
        assert_eq!(migrated["schemaVersion"], 1);
        let backup: Value = serde_json::from_slice(
            &fs::read(path.with_extension("pre-migration.bak")).expect("read backup"),
        )
        .expect("parse backup");
        assert!(backup.get("schemaVersion").is_none());
    }
}
