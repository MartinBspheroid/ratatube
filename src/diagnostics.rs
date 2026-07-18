//! Diagnostic file handling that must preserve prior incident evidence.

use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::error::Result;

/// Rotate logs after five MiB to keep diagnostic storage bounded.
pub const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

/// Open a restrictive append-only log, rotating one prior generation.
pub fn open_log(path: &Path) -> Result<File> {
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.len() >= MAX_LOG_BYTES) {
        let rotated = path.with_extension("log.1");
        if rotated.exists() {
            std::fs::remove_file(&rotated)?;
        }
        std::fs::rename(path, rotated)?;
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn opening_log_appends_without_truncating_prior_evidence() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("ytm-tui.log");
        std::fs::write(&path, b"prior crash\n").expect("seed log");

        let mut file = open_log(&path).expect("open append log");
        file.write_all(b"next launch\n").expect("append");
        drop(file);

        assert_eq!(
            std::fs::read(&path).expect("read log"),
            b"prior crash\nnext launch\n"
        );
    }

    #[test]
    fn oversized_log_rotates_one_generation() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("ytm-tui.log");
        let seed = File::create(&path).expect("create large log");
        seed.set_len(MAX_LOG_BYTES).expect("size log");
        drop(seed);

        let _file = open_log(&path).expect("rotate and open");

        assert_eq!(std::fs::metadata(&path).expect("new log").len(), 0);
        assert_eq!(
            std::fs::metadata(path.with_extension("log.1"))
                .expect("rotated log")
                .len(),
            MAX_LOG_BYTES
        );
    }

    #[cfg(unix)]
    #[test]
    fn new_log_is_not_group_or_world_readable() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("ytm-tui.log");
        let _file = open_log(&path).expect("open log");
        let mode = std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
    }
}
