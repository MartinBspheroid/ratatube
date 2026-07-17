//! Platform-appropriate storage locations (PRD section 11.1).

use std::path::PathBuf;

use directories::ProjectDirs;

use crate::error::{AppError, Result};

/// Resolved filesystem layout for the application.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Data directory, e.g. `~/.local/share/ytm-tui`.
    pub data_dir: PathBuf,
    /// Config directory, e.g. `~/.config/ytm-tui`.
    pub config_dir: PathBuf,
}

impl AppPaths {
    /// Resolve paths using platform conventions.
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("", "", "ytm-tui").ok_or_else(|| {
            AppError::Config("could not resolve platform data directories".to_string())
        })?;
        Ok(Self {
            data_dir: dirs.data_dir().to_path_buf(),
            config_dir: dirs.config_dir().to_path_buf(),
        })
    }

    /// Override the data directory (used by tests and `--data-dir`).
    pub fn with_data_dir(data_dir: PathBuf) -> Self {
        let config_dir = data_dir.clone();
        Self {
            data_dir,
            config_dir,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    pub fn library_file(&self) -> PathBuf {
        self.data_dir.join("library.json")
    }

    pub fn queue_file(&self) -> PathBuf {
        self.data_dir.join("queue.json")
    }

    pub fn history_file(&self) -> PathBuf {
        self.data_dir.join("history.json")
    }

    /// Directory holding one JSON file per playlist (PRD 11.1).
    pub fn playlists_dir(&self) -> PathBuf {
        self.data_dir.join("playlists")
    }

    pub fn playlist_file(&self, id: &str) -> PathBuf {
        self.playlists_dir().join(format!("{id}.json"))
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("ytm-tui.log")
    }

    /// Create all required directories.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(self.playlists_dir())?;
        Ok(())
    }
}
