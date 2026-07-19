//! High-level playback control built on the mpv IPC client (PRD 10.3).

use std::time::Duration;

use serde_json::json;

use crate::error::{AppError, Result};
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

    fn queue_command(&self, command: Vec<serde_json::Value>) -> Result<()> {
        self.command_tx
            .send(command)
            .map_err(|_| AppError::MpvIpc("command worker has stopped".to_string()))
    }

    /// Queue a load command; acknowledgement is handled by the worker.
    pub fn queue_load(&self, stream_url: &str, title: &str) -> Result<()> {
        self.queue_load_at(stream_url, title, None, false)
    }

    /// Queue a load with optional start position and pause state.
    pub fn queue_load_at(
        &self,
        stream_url: &str,
        title: &str,
        start_seconds: Option<f64>,
        paused: bool,
    ) -> Result<()> {
        self.queue_command(load_command(stream_url, title, start_seconds, paused))
    }

    pub fn queue_seek_by(&self, seconds: i64) -> Result<()> {
        self.queue_command(vec![json!("seek"), json!(seconds), json!("relative")])
    }

    pub fn queue_seek_to(&self, seconds: f64) -> Result<()> {
        self.queue_command(vec![json!("seek"), json!(seconds), json!("absolute")])
    }

    pub fn queue_toggle_pause(&self) -> Result<()> {
        self.queue_command(vec![json!("cycle"), json!("pause")])
    }

    pub fn queue_adjust_volume(&mut self, delta: i8) -> Result<()> {
        let next = (self.snapshot.volume as i16 + i16::from(delta)).clamp(0, 100) as u8;
        self.snapshot.volume = next;
        // Apply the delta inside mpv. An observed property-change event for an
        // earlier key press can arrive before this queued command executes;
        // sending an absolute value would then repeatedly reset volume to 2%.
        self.queue_command(vec![json!("add"), json!("volume"), json!(delta)])
    }

    pub fn queue_toggle_mute(&self) -> Result<()> {
        self.queue_command(vec![json!("cycle"), json!("mute")])
    }

    pub fn queue_set_speed(&mut self, speed: f64) -> Result<()> {
        let speed = speed.clamp(0.25, 4.0);
        self.snapshot.speed = speed;
        self.queue_command(vec![json!("set_property"), json!("speed"), json!(speed)])
    }

    pub fn queue_stop(&self) -> Result<()> {
        self.queue_command(vec![json!("stop")])
    }

    pub fn queue_quit(&self) -> Result<()> {
        self.queue_command(vec![json!("quit")])
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
        self.ipc
            .command(load_command(stream_url, title, start_seconds, paused))
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
        let volume = volume.min(100);
        self.ipc.set_property("volume", json!(volume)).await?;
        // Keep rapid repeated key presses cumulative even before mpv emits
        // the observed property-change event for the preceding command.
        self.snapshot.volume = volume;
        Ok(())
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

impl Drop for PlaybackController {
    fn drop(&mut self) {
        self.command_task.abort();
    }
}

fn load_command(
    stream_url: &str,
    title: &str,
    start_seconds: Option<f64>,
    paused: bool,
) -> Vec<serde_json::Value> {
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
    vec![
        json!("loadfile"),
        json!(stream_url),
        json!("replace"),
        json!(-1),
        json!(options),
    ]
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;
    use tokio::sync::mpsc;

    use super::PlaybackController;
    use crate::playback::{PlaybackEvent, ipc::MpvIpc};

    #[tokio::test]
    async fn repeated_volume_adjustments_accumulate_before_mpv_events_arrive() {
        let temp = tempfile::tempdir().expect("temp dir");
        let socket = temp.path().join("mpv.sock");
        let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept IPC client");
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();
            while let Some(line) = lines.next_line().await.expect("read request") {
                let request: serde_json::Value = serde_json::from_str(&line).expect("request json");
                let request_id = request["request_id"].as_u64().expect("request id");
                writer
                    .write_all(
                        format!("{{\"request_id\":{request_id},\"error\":\"success\"}}\n")
                            .as_bytes(),
                    )
                    .await
                    .expect("write acknowledgement");
            }
        });
        let (event_tx, _event_rx) = mpsc::channel(8);
        let ipc = MpvIpc::connect(&socket, event_tx).await.expect("connect");
        let mut controller = PlaybackController::new(ipc);
        controller.on_event(&PlaybackEvent::VolumeChanged(50.0));

        controller.adjust_volume(2).await.expect("first increase");
        controller.adjust_volume(2).await.expect("second increase");

        assert_eq!(controller.snapshot().volume, 54);
        server.abort();
    }

    #[tokio::test]
    async fn queued_volume_adjustments_remain_relative_after_a_stale_event() {
        let temp = tempfile::tempdir().expect("temp dir");
        let socket = temp.path().join("mpv.sock");
        let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept IPC client");
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();
            let mut commands = Vec::new();
            while commands.len() < 2 {
                let line = lines
                    .next_line()
                    .await
                    .expect("read request")
                    .expect("request line");
                let request: serde_json::Value = serde_json::from_str(&line).expect("request json");
                commands.push(request["command"].clone());
                let request_id = request["request_id"].as_u64().expect("request id");
                writer
                    .write_all(
                        format!("{{\"request_id\":{request_id},\"error\":\"success\"}}\n")
                            .as_bytes(),
                    )
                    .await
                    .expect("write acknowledgement");
            }
            commands
        });
        let (event_tx, _event_rx) = mpsc::channel(8);
        let ipc = MpvIpc::connect(&socket, event_tx).await.expect("connect");
        let mut controller = PlaybackController::new(ipc);

        controller.queue_adjust_volume(2).expect("first increase");
        controller.on_event(&PlaybackEvent::VolumeChanged(0.0));
        controller.queue_adjust_volume(2).expect("second increase");

        let commands = server.await.expect("server");
        assert_eq!(
            commands,
            vec![
                serde_json::json!(["add", "volume", 2]),
                serde_json::json!(["add", "volume", 2]),
            ]
        );
    }

    #[tokio::test]
    async fn queued_control_returns_before_delayed_acknowledgement() {
        let temp = tempfile::tempdir().expect("temp dir");
        let socket = temp.path().join("mpv.sock");
        let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept IPC client");
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();
            let line = lines
                .next_line()
                .await
                .expect("read request")
                .expect("request line");
            let request: serde_json::Value = serde_json::from_str(&line).expect("request json");
            let request_id = request["request_id"].as_u64().expect("request id");
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            writer
                .write_all(
                    format!("{{\"request_id\":{request_id},\"error\":\"success\"}}\n").as_bytes(),
                )
                .await
                .expect("write acknowledgement");
        });
        let (event_tx, _event_rx) = mpsc::channel(8);
        let ipc = MpvIpc::connect(&socket, event_tx).await.expect("connect");
        let controller = PlaybackController::new(ipc);
        let started = std::time::Instant::now();

        controller.queue_seek_to(12.0).expect("queue seek");

        assert!(started.elapsed() < std::time::Duration::from_millis(50));
        server.await.expect("server");
    }
}
