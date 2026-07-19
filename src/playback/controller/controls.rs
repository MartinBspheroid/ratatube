use serde_json::json;

use super::{PlaybackController, commands::load_command};
use crate::error::Result;

impl PlaybackController {
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
