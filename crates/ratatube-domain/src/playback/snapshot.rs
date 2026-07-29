//! Observable playback state and the transport constants that shape it.

use std::time::Duration;

use crate::playback::events::{AudioLevels, PlaybackStatus};

/// Small seek step in seconds (PRD 10.3).
pub const SEEK_SMALL: Duration = Duration::from_secs(5);
/// Large seek step in seconds.
pub const SEEK_LARGE: Duration = Duration::from_secs(30);
/// Position threshold for Previous-restarts-current behavior (PRD 10.6).
pub const PREVIOUS_RESTART_THRESHOLD: Duration = Duration::from_secs(5);

/// Observable playback state snapshot for the UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub status: PlaybackStatus,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub volume: u8,
    pub muted: bool,
    /// Playback speed multiplier (1.0 = normal).
    pub speed: f64,
    /// Latest real audio levels while playing; `None` when idle or paused.
    #[serde(default)]
    pub audio_levels: Option<AudioLevels>,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::default(),
            position_seconds: 0.0,
            duration_seconds: None,
            volume: 0,
            muted: false,
            speed: 1.0,
            audio_levels: None,
        }
    }
}
