//! mpv JSON IPC client.
//!
//! mpv speaks newline-delimited JSON over a unix domain socket (PRD 10.3).
//! One owner task reads events; commands carry a monotonically increasing
//! `request_id` so responses can be correlated (PRD section 15).

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::playback::events::PlaybackEvent;

/// Raw event frame from mpv.
#[derive(Debug, Deserialize)]
struct MpvFrame {
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Low-level IPC connection to a running mpv instance.
pub struct MpvIpc {
    writer: OwnedWriteHalf,
    request_counter: AtomicU64,
}

impl MpvIpc {
    /// Connect to the socket and split into command handle + event stream.
    pub async fn connect(
        socket_path: &Path,
        event_tx: mpsc::Sender<PlaybackEvent>,
    ) -> Result<Self> {
        let stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| AppError::MpvIpc(format!("connect {}: {e}", socket_path.display())))?;
        let (read_half, writer) = stream.into_split();
        tokio::spawn(read_events(read_half, event_tx));
        Ok(Self {
            writer,
            request_counter: AtomicU64::new(1),
        })
    }

    /// Send a raw mpv command (array form), e.g. `["seek", 5, "relative"]`.
    pub async fn command(&mut self, args: Vec<Value>) -> Result<u64> {
        let request_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let frame = json!({ "command": args, "request_id": request_id });
        let mut line = serde_json::to_vec(&frame)?;
        line.push(b'\n');
        self.writer
            .write_all(&line)
            .await
            .map_err(|e| AppError::MpvIpc(format!("write: {e}")))?;
        Ok(request_id)
    }

    /// Set an mpv property.
    pub async fn set_property(&mut self, name: &str, value: Value) -> Result<()> {
        self.command(vec![json!("set_property"), json!(name), value])
            .await?;
        Ok(())
    }

    /// Ask mpv to report a property once (value arrives as an event).
    pub async fn get_property(&mut self, name: &str) -> Result<()> {
        self.command(vec![json!("get_property"), json!(name)])
            .await?;
        Ok(())
    }

    /// Subscribe to a property change.
    pub async fn observe_property(&mut self, id: u64, name: &str) -> Result<()> {
        self.command(vec![json!("observe_property"), json!(id), json!(name)])
            .await?;
        Ok(())
    }
}

/// Owner task: decode newline-delimited JSON frames into playback events.
async fn read_events(reader: OwnedReadHalf, event_tx: mpsc::Sender<PlaybackEvent>) {
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Some(event) = parse_frame(&line)
                    && event_tx.send(event).await.is_err()
                {
                    break;
                }
            }
            Ok(None) => {
                let _ = event_tx.send(PlaybackEvent::Shutdown).await;
                break;
            }
            Err(err) => {
                tracing::warn!(?err, "mpv IPC read error");
                let _ = event_tx.send(PlaybackEvent::Shutdown).await;
                break;
            }
        }
    }
}

/// Translate one raw mpv frame into a typed event.
fn parse_frame(line: &str) -> Option<PlaybackEvent> {
    let frame: MpvFrame = serde_json::from_str(line).ok()?;
    // Command responses carry an `error` field; surface failures.
    if let Some(error) = &frame.error
        && error != "success"
    {
        return Some(PlaybackEvent::PlaybackError(error.clone()));
    }
    let event = frame.event.as_deref()?;
    match event {
        "playback-restart" => Some(PlaybackEvent::Started),
        "end-file" => Some(PlaybackEvent::EndFile {
            reason: frame.reason.unwrap_or_else(|| "unknown".to_string()),
        }),
        "property-change" => match (frame.name.as_deref(), frame.data) {
            (Some("time-pos"), Some(Value::Number(n))) => {
                n.as_f64().map(PlaybackEvent::PositionChanged)
            }
            (Some("duration"), Some(Value::Number(n))) => {
                n.as_f64().map(PlaybackEvent::DurationChanged)
            }
            (Some("pause"), Some(Value::Bool(b))) => Some(PlaybackEvent::PauseChanged(b)),
            (Some("volume"), Some(Value::Number(n))) => {
                n.as_f64().map(PlaybackEvent::VolumeChanged)
            }
            (Some("mute"), Some(Value::Bool(b))) => Some(PlaybackEvent::MuteChanged(b)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_end_file() {
        let event = parse_frame(r#"{"event":"end-file","reason":"eof"}"#).expect("event");
        assert_eq!(
            event,
            PlaybackEvent::EndFile {
                reason: "eof".to_string()
            }
        );
    }

    #[test]
    fn parses_position_change() {
        let event =
            parse_frame(r#"{"event":"property-change","name":"time-pos","data":12.5}"#).expect("e");
        assert_eq!(event, PlaybackEvent::PositionChanged(12.5));
    }

    #[test]
    fn ignores_unknown_frames() {
        assert!(parse_frame(r#"{"event":"video-reconfig"}"#).is_none());
        assert!(parse_frame("not json").is_none());
    }
}
