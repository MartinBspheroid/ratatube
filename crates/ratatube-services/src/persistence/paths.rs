//! Platform-appropriate storage locations (PRD section 11.1).

use std::path::PathBuf;

use directories::ProjectDirs;

use ratatube_domain::error::{AppError, Result};

/// Application identifier used for platform directories and runtime files.
pub const APP_NAME: &str = "ratatube";
/// Pre-rename identifier; existing installations keep their directories.
const LEGACY_APP_NAME: &str = "ytm-tui";

/// Resolved filesystem layout for the application.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Data directory, e.g. `~/.local/share/ratatube`.
    pub data_dir: PathBuf,
    /// Config directory, e.g. `~/.config/ratatube`.
    pub config_dir: PathBuf,
}

impl AppPaths {
    /// Resolve paths using platform conventions. Installations created
    /// under the pre-rename `ytm-tui` name are grandfathered: when the new
    /// data directory does not exist but the legacy one does, the legacy
    /// directories stay authoritative so no user data moves.
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("", "", APP_NAME).ok_or_else(|| {
            AppError::Config("could not resolve platform data directories".to_string())
        })?;
        if !dirs.data_dir().exists()
            && let Some(legacy) = ProjectDirs::from("", "", LEGACY_APP_NAME)
            && legacy.data_dir().exists()
        {
            return Ok(Self {
                data_dir: legacy.data_dir().to_path_buf(),
                config_dir: legacy.config_dir().to_path_buf(),
            });
        }
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

    /// Last-session snapshot for resume-on-launch.
    pub fn session_file(&self) -> PathBuf {
        self.data_dir.join("session.json")
    }

    /// Directory holding one JSON file per playlist (PRD 11.1).
    pub fn playlists_dir(&self) -> PathBuf {
        self.data_dir.join("playlists")
    }

    pub fn playlist_file(&self, id: &str) -> PathBuf {
        self.playlists_dir().join(format!("{id}.json"))
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("ratatube.log")
    }

    /// Create all required directories.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(self.playlists_dir())?;
        Ok(())
    }
}
