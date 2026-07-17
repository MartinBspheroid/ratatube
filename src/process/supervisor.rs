//! External process supervision (PRD section 14).
//!
//! mpv and yt-dlp are unreliable boundaries: binaries may be missing,
//! output may be invalid, and processes may exit unexpectedly.

use std::path::PathBuf;

use crate::error::{AppError, Result};

/// Outcome of probing one executable.
#[derive(Debug, Clone)]
pub struct DependencyProbe {
    pub name: &'static str,
    pub binary: String,
    pub status: DependencyStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyStatus {
    /// Found in PATH (or at the configured absolute path).
    Found {
        path: PathBuf,
        version: Option<String>,
    },
    Missing,
}

impl DependencyProbe {
    pub fn is_ok(&self) -> bool {
        matches!(self.status, DependencyStatus::Found { .. })
    }
}

/// Report for the `doctor` command (PRD section 18).
#[derive(Debug, Clone, Default)]
pub struct DoctorReport {
    pub dependencies: Vec<DependencyProbe>,
    pub data_dir_ok: Option<bool>,
    pub config_ok: Option<bool>,
    pub ipc_path_ok: Option<bool>,
}

impl DoctorReport {
    pub fn all_ok(&self) -> bool {
        self.dependencies.iter().all(DependencyProbe::is_ok)
            && self.data_dir_ok.unwrap_or(true)
            && self.config_ok.unwrap_or(true)
            && self.ipc_path_ok.unwrap_or(true)
    }
}

/// Verify a required executable exists; map absence to a structured error.
pub fn require(binary: &str) -> Result<PathBuf> {
    which::which(binary).map_err(|_| AppError::MissingDependency(binary.to_string()))
}

/// Probe an executable and try to read its `--version` output.
pub fn probe(name: &'static str, binary: &str) -> DependencyProbe {
    let status = match which::which(binary) {
        Ok(path) => {
            let version = std::process::Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .and_then(|out| {
                    String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .next()
                        .map(str::to_string)
                });
            DependencyStatus::Found { path, version }
        }
        Err(_) => DependencyStatus::Missing,
    };
    DependencyProbe {
        name,
        binary: binary.to_string(),
        status,
    }
}

/// Installation hints shown when a dependency is missing (PRD section 6).
pub fn install_hint(binary: &str) -> &'static str {
    match binary {
        "mpv" => "install via `brew install mpv` (macOS) or your distro package manager",
        "yt-dlp" => "install via `brew install yt-dlp` or `pipx install yt-dlp`",
        "ffmpeg" => "install via `brew install ffmpeg` or your distro package manager",
        _ => "see the tool's documentation",
    }
}
