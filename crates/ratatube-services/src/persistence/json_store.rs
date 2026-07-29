//! Atomic JSON persistence (PRD section 11.4).
//!
//! Writes serialize the full value, write to a *uniquely named* sibling temp
//! file, flush, sync, rename over the original, and finally sync the parent
//! directory so the rename itself survives a crash. Malformed files are never
//! silently reset: reads report the affected file and preserve a `.bak` copy.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde::de::DeserializeOwned;

use ratatube_domain::error::{AppError, Result};

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
    // Unique per write: two writers racing on the same document must not
    // share scratch space, or one can rename the other's partial payload
    // into place.
    let tmp = tmp_path(path);

    let write_result = (|| -> Result<()> {
        fs::write(&tmp, &payload)?;
        // Owner-only before it is published, so the document is never briefly
        // world-readable and does not depend on the data directory's mode to
        // stay private. These files hold listening history and playlists.
        restrict_to_owner(&tmp)?;
        // Flush the data to disk before rename.
        let file = fs::File::open(&tmp)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        // Flush the directory entry too: without this the rename can be lost
        // after a power failure even though the payload was synced.
        sync_dir(parent)?;
        Ok(())
    })();

    if write_result.is_err() {
        // Preserve the previous file; clean up the temp file best effort. The
        // temp path is only ever touched by this call, so removing it after a
        // post-rename failure is a harmless no-op.
        let _ = fs::remove_file(&tmp);
    }
    write_result
}

/// Restrict a file to its owner before it is published under its real name.
#[cfg(unix)]
fn restrict_to_owner(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// Non-unix platforms have no equivalent mode to set here; the containing
/// directory remains the access boundary.
#[cfg(not(unix))]
fn restrict_to_owner(_path: &Path) -> Result<()> {
    Ok(())
}

/// Fsync a directory so a rename inside it becomes durable.
///
/// On Unix (the platforms this project targets) a directory can be opened
/// read-only and fsynced, which is exactly what POSIX requires to make a
/// rename survive a crash. Errors propagate like every other IO failure in
/// [`atomic_write`].
#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<()> {
    let handle = fs::File::open(dir)?;
    handle.sync_all()?;
    Ok(())
}

/// Non-Unix fallback: Windows cannot open a directory as a regular file, and
/// it offers no per-directory flush, so there is nothing to sync here. This
/// is a genuine gap rather than a silent one — the durability guarantee below
/// Unix is whatever the filesystem gives us for the rename itself.
#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<()> {
    Ok(())
}

/// Temp file name for one write attempt.
///
/// The process id plus a monotonic counter makes the name unique across
/// threads within a process and across processes on the same machine, without
/// pulling in a new dependency for random names.
///
/// A hard crash mid-write therefore leaves an orphan `*.tmp.<pid>.<seq>` that
/// nothing reuses. That is deliberate: a sweeper for those siblings could not
/// tell an orphan from another process's in-flight write without racing a
/// liveness check, and deleting a live writer's scratch file is exactly the
/// corruption this unique naming removed. A few stale bytes after a crash is
/// the cheaper failure. Reap them from the data directory at startup if it
/// ever matters — not from here.
fn tmp_path(path: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".tmp.{}.{}", std::process::id(), seq));
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

    /// Every temp file this module can create is a sibling whose name
    /// contains `.tmp`; a write must leave none of them behind. This is
    /// stronger than probing one predicted name, because temp names are
    /// unique per write and unpredictable from the outside.
    fn stray_tmp_files(dir: &Path) -> Vec<PathBuf> {
        fs::read_dir(dir)
            .expect("read_dir")
            .map(|entry| entry.expect("entry").path())
            .filter(|entry| {
                entry
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(".tmp"))
            })
            .collect()
    }

    #[test]
    fn atomic_write_and_read_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        atomic_write(&path, &serde_json::json!({"a": 1})).expect("write");
        assert_eq!(
            stray_tmp_files(dir.path()),
            Vec::<PathBuf>::new(),
            "temp file must be renamed away"
        );
        let value: serde_json::Value = read(&path).expect("read");
        assert_eq!(value["a"], 1);
    }

    #[cfg(unix)]
    #[test]
    fn written_documents_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        atomic_write(&path, &serde_json::json!({"a": 1})).expect("write");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "documents hold listening history; they must not be group/other readable"
        );

        // A rewrite must not widen the mode either.
        atomic_write(&path, &serde_json::json!({"a": 2})).expect("rewrite");
        let mode = fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn concurrent_writes_never_publish_a_partial_document() {
        const WRITERS: usize = 8;
        const ROUNDS: usize = 40;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("doc.json");
        // Distinct sizes per writer so a shared scratch file shows up as a
        // truncated or blended payload instead of hiding behind equal lengths.
        let payloads: Vec<(usize, String)> = (0..WRITERS)
            .map(|writer| (writer, "x".repeat(64 * 1024 * (writer + 1))))
            .collect();

        for _ in 0..ROUNDS {
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(WRITERS));
            let handles: Vec<_> = payloads
                .iter()
                .map(|(writer, filler)| {
                    let path = path.clone();
                    let barrier = std::sync::Arc::clone(&barrier);
                    let value = serde_json::json!({"who": writer, "filler": filler});
                    std::thread::spawn(move || {
                        // Line the writers up so their write/rename windows
                        // actually overlap.
                        barrier.wait();
                        atomic_write(&path, &value)
                    })
                })
                .collect();
            for handle in handles {
                handle.join().expect("thread").expect("write");
            }

            // Whichever writer won the rename, the published document must be
            // exactly one complete payload — never truncated or interleaved.
            let stored: serde_json::Value = read(&path).expect("read");
            let who = stored["who"].as_u64().expect("who") as usize;
            assert_eq!(
                stored["filler"].as_str().expect("filler"),
                payloads[who].1,
                "writer {who} published a corrupted payload"
            );
            assert_eq!(
                stray_tmp_files(dir.path()),
                Vec::<PathBuf>::new(),
                "concurrent writes must not leak temp files"
            );
        }
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
