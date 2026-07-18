//! High-level playback control built on the mpv IPC client (PRD 10.3).

use std::time::Duration;

use serde_json::json;

use crate::error::Result;
use crate::playback::events::{PlaybackEvent, PlaybackStatus};
use crate::playback::ipc::MpvIpc;

/// Small seek step in seconds (PRD 10.3).
pub const SEEK_SMALL: Duration = Duration::from_secs(5);
/// Large seek step in seconds.
pub const SEEK_LARGE: Duration = Duration::from_secs(30);
/// Position threshold for Previous-restarts-current behavior (PRD 10.6).
pub const PREVIOUS_RESTART_THRESHOLD: Duration = Duration::from_secs(5);

/// Observable playback state snapshot for the UI.
#[derive(Debug, Clone)]
pub struct PlaybackSnapshot {
    pub status: PlaybackStatus,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub volume: u8,
    pub muted: bool,
    /// Playback speed multiplier (1.0 = normal).
    pub speed: f64,
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
        }
    }
}

/// Facade over [`MpvIpc`] exposing the required playback actions.
pub struct PlaybackController {
    ipc: MpvIpc,
    snapshot: PlaybackSnapshot,
}

impl PlaybackController {
    pub fn new(ipc: MpvIpc) -> Self {
        Self {
            ipc,
            snapshot: PlaybackSnapshot::default(),
        }
    }

    pub fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }

    /// Apply an incoming mpv event to the local snapshot.
    pub fn on_event(&mut self, event: &PlaybackEvent) {
        match event {
            PlaybackEvent::Started => self.snapshot.status = PlaybackStatus::Playing,
            PlaybackEvent::PositionChanged(p) => self.snapshot.position_seconds = *p,
            PlaybackEvent::DurationChanged(d) => self.snapshot.duration_seconds = Some(*d),
            PlaybackEvent::PauseChanged(paused) => {
                self.snapshot.status = if *paused {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Playing
                };
            }
            PlaybackEvent::VolumeChanged(v) => {
                self.snapshot.volume = (*v).clamp(0.0, 100.0) as u8;
            }
            PlaybackEvent::MuteChanged(m) => self.snapshot.muted = *m,
            PlaybackEvent::SpeedChanged(s) => self.snapshot.speed = *s,
            PlaybackEvent::EndFile { .. } => self.snapshot.status = PlaybackStatus::Stopped,
            PlaybackEvent::PlaybackError(_) | PlaybackEvent::Shutdown => {
                self.snapshot.status = PlaybackStatus::Idle;
            }
            PlaybackEvent::Connected => {}
        }
    }

    /// Subscribe to the properties the UI needs.
    pub async fn observe_defaults(&mut self) -> Result<()> {
        self.ipc.observe_property(1, "time-pos").await?;
        self.ipc.observe_property(2, "duration").await?;
        self.ipc.observe_property(3, "pause").await?;
        self.ipc.observe_property(4, "volume").await?;
        self.ipc.observe_property(5, "mute").await?;
        self.ipc.observe_property(6, "speed").await?;
        Ok(())
    }

    /// Set the playback speed multiplier.
    pub async fn set_speed(&mut self, speed: f64) -> Result<()> {
        self.ipc
            .set_property("speed", json!(speed.clamp(0.25, 4.0)))
            .await
    }

    /// Load and play a resolved stream URL with a display title.
    pub async fn load(&mut self, stream_url: &str, title: &str) -> Result<()> {
        self.load_at(stream_url, title, None, false).await
    }

    /// Load a stream with optional start position and paused state applied
    /// atomically via `loadfile` options — no seek/pause race after load.
    pub async fn load_at(
        &mut self,
        stream_url: &str,
        title: &str,
        start_seconds: Option<f64>,
        paused: bool,
    ) -> Result<()> {
        let mut options = serde_json::Map::new();
        options.insert("force-media-title".to_string(), json!(title));
        if let Some(start) = start_seconds
            && start > 0.0
        {
            options.insert("start".to_string(), json!(format!("{start:.1}")));
        }
        options.insert(
            "pause".to_string(),
            json!(if paused { "yes" } else { "no" }),
        );
        self.ipc
            .command(vec![
                json!("loadfile"),
                json!(stream_url),
                json!("replace"),
                json!(-1),
                json!(options),
            ])
            .await?;
        Ok(())
    }

    pub async fn pause(&mut self) -> Result<()> {
        self.ipc.set_property("pause", json!(true)).await
    }

    pub async fn resume(&mut self) -> Result<()> {
        self.ipc.set_property("pause", json!(false)).await
    }

    pub async fn toggle_pause(&mut self) -> Result<()> {
        self.ipc
            .command(vec![json!("cycle"), json!("pause")])
            .await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.ipc.command(vec![json!("stop")]).await?;
        Ok(())
    }

    /// Relative seek in seconds (negative seeks backward).
    pub async fn seek_by(&mut self, seconds: i64) -> Result<()> {
        self.ipc
            .command(vec![json!("seek"), json!(seconds), json!("relative")])
            .await?;
        Ok(())
    }

    /// Absolute seek in seconds.
    pub async fn seek_to(&mut self, seconds: f64) -> Result<()> {
        self.ipc
            .command(vec![json!("seek"), json!(seconds), json!("absolute")])
            .await?;
        Ok(())
    }

    pub async fn set_volume(&mut self, volume: u8) -> Result<()> {
        self.ipc
            .set_property("volume", json!(volume.min(100)))
            .await
    }

    /// Adjust volume by a signed delta, clamped to 0-100.
    pub async fn adjust_volume(&mut self, delta: i8) -> Result<()> {
        let next = (self.snapshot.volume as i16 + i16::from(delta)).clamp(0, 100) as u8;
        self.set_volume(next).await
    }

    pub async fn toggle_mute(&mut self) -> Result<()> {
        self.ipc
            .command(vec![json!("cycle"), json!("mute")])
            .await?;
        Ok(())
    }

    /// Ask mpv to quit gracefully.
    pub async fn quit(&mut self) -> Result<()> {
        self.ipc.command(vec![json!("quit")]).await?;
        Ok(())
    }
}
