//! High-level playback control built on the mpv IPC client (PRD 10.3).

use std::time::Duration;

use crate::playback::events::{PlaybackEvent, PlaybackStatus};
use crate::playback::ipc::MpvIpc;

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
    pub audio_levels: Option<crate::playback::events::AudioLevels>,
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

/// Facade over [`MpvIpc`] exposing the required playback actions.
pub struct PlaybackController {
    ipc: MpvIpc,
    snapshot: PlaybackSnapshot,
    command_tx: tokio::sync::mpsc::UnboundedSender<Vec<serde_json::Value>>,
    command_task: tokio::task::JoinHandle<()>,
}

impl PlaybackController {
    pub fn new(ipc: MpvIpc) -> Self {
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let command_ipc = ipc.clone();
        let event_tx = ipc.event_sender();
        let command_task = tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                if let Err(err) = command_ipc.command(command).await {
                    let _ = event_tx
                        .send(PlaybackEvent::PlaybackError(err.to_string()))
                        .await;
                }
            }
        });
        Self {
            ipc,
            snapshot: PlaybackSnapshot::default(),
            command_tx,
            command_task,
        }
    }

    pub fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }

    /// Apply an incoming mpv event to the local snapshot.
    pub fn on_event(&mut self, event: &PlaybackEvent) {
        match event {
            PlaybackEvent::Started => self.snapshot.status = PlaybackStatus::Playing,
            PlaybackEvent::FileLoaded => {}
            PlaybackEvent::PositionChanged(p) => self.snapshot.position_seconds = *p,
            PlaybackEvent::DurationChanged(d) => self.snapshot.duration_seconds = Some(*d),
            PlaybackEvent::PauseChanged(paused) => {
                self.snapshot.status = if *paused {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Playing
                };
                if *paused {
                    self.snapshot.audio_levels = None;
                }
            }
            PlaybackEvent::VolumeChanged(v) => {
                self.snapshot.volume = (*v).clamp(0.0, 100.0) as u8;
            }
            PlaybackEvent::MuteChanged(m) => self.snapshot.muted = *m,
            PlaybackEvent::SpeedChanged(s) => self.snapshot.speed = *s,
            PlaybackEvent::AudioLevels(levels) => self.snapshot.audio_levels = Some(*levels),
            PlaybackEvent::EndFile { .. } => {
                self.snapshot.status = PlaybackStatus::Stopped;
                self.snapshot.audio_levels = None;
            }
            PlaybackEvent::PlaybackError(_) | PlaybackEvent::Shutdown => {
                self.snapshot.status = PlaybackStatus::Idle;
                self.snapshot.audio_levels = None;
            }
            PlaybackEvent::Connected => {}
        }
    }
}

impl Drop for PlaybackController {
    fn drop(&mut self) {
        self.command_task.abort();
    }
}

mod commands;
mod controls;
#[cfg(test)]
mod tests;
