//! mpv process lifecycle (PRD 10.3, section 14).
//!
//! mpv is persistent for the application session, launched with no video,
//! JSON IPC, idle mode, and redirected terminal output.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::process::{Child, Command};

use crate::error::{AppError, Result};

/// A running mpv child process plus its IPC socket path.
pub struct MpvProcess {
    child: Child,
    socket_path: PathBuf,
}

impl MpvProcess {
    /// Spawn mpv in audio-only idle mode with JSON IPC on `socket_path`.
    pub fn spawn(binary: &str, socket_path: &Path, initial_volume: u8) -> Result<Self> {
        if socket_path.exists() {
            // Stale socket from a previous run.
            let _ = std::fs::remove_file(socket_path);
        }
        let child = Command::new(binary)
            .arg("--no-video")
            .arg("--idle=yes")
            .arg("--no-terminal")
            .arg("--really-quiet")
            .arg(format!("--input-ipc-server={}", socket_path.display()))
            .arg(format!("--volume={initial_volume}"))
            .arg("--ytdl=no")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    AppError::MissingDependency(binary.to_string())
                } else {
                    AppError::Process {
                        command: binary.to_string(),
                        message: err.to_string(),
                    }
                }
            })?;
        Ok(Self {
            child,
            socket_path: socket_path.to_path_buf(),
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Wait until the IPC socket appears or mpv exits.
    pub async fn wait_for_socket(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if self.socket_path.exists() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Err(AppError::Timeout(
            "mpv IPC socket did not appear".to_string(),
        ))
    }

    /// Graceful shutdown: ask mpv to quit, wait briefly, then kill (PRD 14).
    pub async fn shutdown(&mut self) {
        match tokio::time::timeout(Duration::from_secs(2), self.child.wait()).await {
            Ok(_) => {}
            Err(_) => {
                let _ = self.child.kill().await;
            }
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
