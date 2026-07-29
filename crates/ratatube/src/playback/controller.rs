//! High-level playback control built on the mpv IPC client (PRD 10.3).

use crate::playback::events::{PlaybackEvent, PlaybackStatus};
use crate::playback::ipc::MpvIpc;
use crate::playback::snapshot::PlaybackSnapshot;

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
