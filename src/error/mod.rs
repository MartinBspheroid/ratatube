//! Crate-wide error types.
//!
//! Errors are categorized per PRD section 16: user-facing recoverable errors
//! surface as inline notices, while system errors require a persistent panel
//! with recovery guidance.

use std::path::PathBuf;

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, AppError>;

/// Top-level application error.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("storage error at {path}: {message}")]
    Storage { path: PathBuf, message: String },

    #[error("persisted data at {0} is malformed; a backup was preserved")]
    MalformedData(PathBuf),

    #[error("schema version {found} at {path} is newer than supported version {supported}")]
    UnsupportedSchema {
        path: PathBuf,
        found: u32,
        supported: u32,
    },

    #[error("required executable `{0}` was not found in PATH")]
    MissingDependency(String),

    #[error("subprocess `{command}` failed: {message}")]
    Process { command: String, message: String },

    #[error("mpv IPC error: {0}")]
    MpvIpc(String),

    #[error("mpv playback error: {0}")]
    MpvPlayback(String),

    #[error("yt-dlp error: {0}")]
    YtDlp(String),

    #[error("media could not be resolved: {0}")]
    Resolve(String),

    #[error("track is unavailable: {0}")]
    Unavailable(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("operation timed out: {0}")]
    Timeout(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Coarse classification used by the UI to decide between an inline notice
/// and a persistent error panel (PRD section 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Recoverable, shown inline or as a transient notification.
    UserFacing,
    /// System-level, shown in a persistent error panel with guidance.
    System,
}

impl AppError {
    /// Classify this error for presentation purposes.
    pub fn category(&self) -> ErrorCategory {
        match self {
            AppError::Config(_)
            | AppError::Storage { .. }
            | AppError::MalformedData(_)
            | AppError::UnsupportedSchema { .. }
            | AppError::MissingDependency(_)
            | AppError::MpvIpc(_)
            | AppError::Io(_) => ErrorCategory::System,
            AppError::Process { .. }
            | AppError::MpvPlayback(_)
            | AppError::YtDlp(_)
            | AppError::Resolve(_)
            | AppError::Unavailable(_)
            | AppError::InvalidUrl(_)
            | AppError::Timeout(_)
            | AppError::Json(_) => ErrorCategory::UserFacing,
        }
    }
}
