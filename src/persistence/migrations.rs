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
pub const PLAYLIST_SCHEMA_VERSION: u32 = 1;
pub const QUEUE_SCHEMA_VERSION: u32 = 1;
pub const HISTORY_SCHEMA_VERSION: u32 = 1;

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

    // No migrations exist yet beyond version 1; future migrations chain here.
    let migrated = raw;
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
}
