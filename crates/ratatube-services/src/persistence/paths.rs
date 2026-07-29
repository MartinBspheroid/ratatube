//! Platform-appropriate storage locations (PRD section 11.1).

use std::path::{Path, PathBuf};

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

    /// Create all required directories, owner-only.
    ///
    /// These directories hold the playlists, the session snapshot and the
    /// full listening history, so on a multi-user machine they must not be
    /// readable by other local users. `create_dir_all` used the process
    /// umask (typically 0o755); every directory is now created with mode
    /// 0o700 instead. The files inside stay at their umask mode, which is
    /// enough: an owner-only directory cannot be traversed by another user,
    /// so the files it contains are unreachable regardless of their own bits.
    pub fn ensure_dirs(&self) -> Result<()> {
        create_private_dir(&self.data_dir)?;
        create_private_dir(&self.config_dir)?;
        create_private_dir(&self.playlists_dir())?;
        Ok(())
    }
}

/// Create `dir` (with any missing parents) and make sure neither group nor
/// other can reach it.
fn create_private_dir(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        restrict_to_owner(dir);
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(dir)?;
    Ok(())
}

/// Clear the group and other bits of an existing directory.
///
/// Deliberate choice: a directory left behind by an earlier install (created
/// with the umask, so usually 0o755) *is* tightened in place, not merely
/// warned about. Warning only would leave every upgrading user's playlists
/// and listening history world-readable forever, and 0o700 on an
/// application-private directory cannot break anything the application does.
/// Two guard rails keep that from being a destructive surprise: only the
/// group/other bits are cleared, so a deliberately stricter mode such as
/// 0o500 survives untouched; and a failed chmod is logged rather than fatal,
/// because refusing to start would not un-share files that are already
/// exposed — it would only take the player away from the user.
#[cfg(unix)]
fn restrict_to_owner(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let Ok(metadata) = std::fs::metadata(dir) else {
        return;
    };
    let mode = metadata.permissions().mode();
    if mode & 0o077 == 0 {
        return;
    }
    match std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode & !0o077)) {
        Ok(()) => tracing::info!(
            dir = %dir.display(),
            from = format!("{:o}", mode & 0o777),
            "tightened directory to owner-only"
        ),
        Err(err) => tracing::warn!(
            dir = %dir.display(),
            ?err,
            "could not tighten directory to owner-only; contents may be readable by other local users"
        ),
    }
}
