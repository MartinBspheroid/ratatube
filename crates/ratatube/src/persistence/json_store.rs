//! Atomic JSON persistence (PRD section 11.4).
//!
//! Writes serialize the full value, write to a sibling temp file, flush,
//! sync, and rename over the original. Malformed files are never silently
//! reset: reads report the affected file and preserve a `.bak` copy.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{AppError, Result};

/// Maximum accepted JSON document size (16 MiB) to bound hostile input
/// (PRD section 19).
/// Largest JSON document the store will read or write (16 MiB).
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

/// Read and deserialize a JSON document.
///
/// On a parse failure the original file is copied to `<path>.bak` for
/// recovery and [`AppError::MalformedData`] is returned; the original is
/// left untouched.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_DOCUMENT_BYTES as u64 {
        return Err(AppError::Storage {
            path: path.to_path_buf(),
            message: format!("file exceeds {} byte limit", MAX_DOCUMENT_BYTES),
        });
    }
    let raw = fs::read_to_string(path)?;
    match serde_json::from_str(&raw) {
        Ok(value) => Ok(value),
        Err(err) => {
            let backup = backup_path(path);
            // Best effort backup; the original is never modified.
            let _ = fs::copy(path, &backup);
            tracing::warn!(?err, ?path, "malformed JSON; backup preserved");
            Err(AppError::MalformedData(path.to_path_buf()))
        }
    }
}

/// Read JSON without creating recovery artifacts; intended for diagnostics.
pub fn read_only<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_DOCUMENT_BYTES as u64 {
        return Err(AppError::Storage {
            path: path.to_path_buf(),
            message: format!("file exceeds {} byte limit", MAX_DOCUMENT_BYTES),
        });
    }
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(AppError::Json)
}

/// Preserve the current document beside `path` for manual recovery.
pub fn preserve_backup(path: &Path) -> Result<PathBuf> {
    let backup = backup_path(path);
    fs::copy(path, &backup)?;
    Ok(backup)
}

/// Atomically write `value` as pretty JSON to `path`.
pub fn atomic_write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().ok_or_else(|| AppError::Storage {
        path: path.to_path_buf(),
        message: "path has no parent directory".to_string(),
    })?;
    fs::create_dir_all(parent)?;

    let payload = serde_json::to_vec_pretty(value)?;
    if payload.len() > MAX_DOCUMENT_BYTES {
        return Err(AppError::Storage {
            path: path.to_path_buf(),
            message: format!("document exceeds {} byte limit", MAX_DOCUMENT_BYTES),
        });
    }
    let tmp = tmp_path(path);

    let write_result = (|| -> Result<()> {
        fs::write(&tmp, &payload)?;
        // Flush to disk before rename.
        let file = fs::File::open(&tmp)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();

    if write_result.is_err() {
        // Preserve the previous file; clean up the temp file best effort.
        let _ = fs::remove_file(&tmp);
    }
    write_result
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    PathBuf::from(bak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_and_read_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        atomic_write(&path, &serde_json::json!({"a": 1})).expect("write");
        assert!(!tmp_path(&path).exists(), "temp file must be renamed away");
        let value: serde_json::Value = read(&path).expect("read");
        assert_eq!(value["a"], 1);
    }

    #[test]
    fn failed_write_preserves_previous_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        atomic_write(&path, &serde_json::json!({"v": "original"})).expect("write");
        // Serialize a value that fails (non-finite float key is fine; use a
        // map with a non-string key via Value is impossible, so force failure
        // by writing to a path whose parent cannot be created).
        let bad_path = dir.path().join("blocked").join("doc.json");
        fs::write(dir.path().join("blocked"), b"file").expect("block dir");
        let result = atomic_write(&bad_path, &serde_json::json!({"v": 2}));
        assert!(result.is_err());
        let value: serde_json::Value = read(&path).expect("read original");
        assert_eq!(value["v"], "original");
    }

    #[test]
    fn malformed_read_preserves_backup_and_original() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        fs::write(&path, b"{ not json").expect("write malformed");
        let result = read::<serde_json::Value>(&path);
        assert!(matches!(result, Err(AppError::MalformedData(_))));
        assert_eq!(fs::read(&path).expect("original"), b"{ not json");
        assert!(backup_path(&path).exists());
    }

    #[test]
    fn oversized_write_is_rejected_and_preserves_previous_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        atomic_write(&path, &serde_json::json!({"v": "original"})).expect("initial write");
        let oversized = "x".repeat(MAX_DOCUMENT_BYTES);

        let result = atomic_write(&path, &serde_json::json!({"v": oversized}));

        assert!(
            matches!(result, Err(AppError::Storage { path: ref error_path, .. }) if error_path == &path)
        );
        let stored: serde_json::Value = read(&path).expect("read original");
        assert_eq!(stored["v"], "original");
    }
}
