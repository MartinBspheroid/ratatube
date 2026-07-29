use serde_json::json;

use super::PlaybackController;
use ratatube_domain::error::{AppError, Result};

impl PlaybackController {
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
}

pub(super) fn load_command(
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
