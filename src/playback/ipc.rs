//! mpv JSON IPC client.
//!
//! mpv speaks newline-delimited JSON over a unix domain socket (PRD 10.3).
//! One owner task reads events; commands carry a monotonically increasing
//! `request_id` so responses can be correlated (PRD section 15).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::error::{AppError, Result};
use crate::playback::events::PlaybackEvent;

type ResponseSender = oneshot::Sender<Result<Value>>;
type PendingRequests = Arc<Mutex<HashMap<u64, ResponseSender>>>;

/// Raw event frame from mpv.
#[derive(Debug, Deserialize)]
struct MpvFrame {
    #[serde(default)]
    request_id: Option<u64>,
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
#[derive(Clone)]
pub struct MpvIpc {
    writer: Arc<Mutex<OwnedWriteHalf>>,
    request_counter: Arc<AtomicU64>,
    pending: PendingRequests,
    command_timeout: Duration,
    event_tx: mpsc::Sender<PlaybackEvent>,
}

impl MpvIpc {
    /// Connect to the socket and split into command handle + event stream.
    pub async fn connect(
        socket_path: &Path,
        event_tx: mpsc::Sender<PlaybackEvent>,
    ) -> Result<Self> {
        Self::connect_with_timeout(socket_path, event_tx, Duration::from_secs(3)).await
    }

    /// Connect using an explicit acknowledgement timeout.
    pub async fn connect_with_timeout(
        socket_path: &Path,
        event_tx: mpsc::Sender<PlaybackEvent>,
        command_timeout: Duration,
    ) -> Result<Self> {
        let stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| AppError::MpvIpc(format!("connect {}: {e}", socket_path.display())))?;
        let (read_half, writer) = stream.into_split();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        tokio::spawn(read_events(read_half, event_tx.clone(), pending.clone()));
        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            request_counter: Arc::new(AtomicU64::new(1)),
            pending,
            command_timeout,
            event_tx,
        })
    }

    /// Clone the playback-event sender used for asynchronous command errors.
    pub fn event_sender(&self) -> mpsc::Sender<PlaybackEvent> {
        self.event_tx.clone()
    }

    /// Send a raw mpv command (array form), e.g. `["seek", 5, "relative"]`.
    pub async fn command(&self, args: Vec<Value>) -> Result<Value> {
        let request_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let (response_tx, response_rx) = oneshot::channel();
        self.pending.lock().await.insert(request_id, response_tx);
        let frame = json!({ "command": args, "request_id": request_id });
        let mut line = serde_json::to_vec(&frame)?;
        line.push(b'\n');
        if let Err(err) = self.writer.lock().await.write_all(&line).await {
            self.pending.lock().await.remove(&request_id);
            return Err(AppError::MpvIpc(format!("write: {err}")));
        }
        match tokio::time::timeout(self.command_timeout, response_rx).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => Err(AppError::MpvIpc(
                "response router closed before acknowledgement".to_string(),
            )),
            Err(_) => {
                self.pending.lock().await.remove(&request_id);
                Err(AppError::Timeout(format!(
                    "mpv command {request_id} was not acknowledged within {:?}",
                    self.command_timeout
                )))
            }
        }
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
async fn read_events(
    reader: OwnedReadHalf,
    event_tx: mpsc::Sender<PlaybackEvent>,
    pending: PendingRequests,
) {
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let frame = match serde_json::from_str::<MpvFrame>(&line) {
                    Ok(frame) => frame,
                    Err(err) => {
                        fail_pending(&pending, &format!("malformed mpv frame: {err}")).await;
                        let _ = event_tx
                            .send(PlaybackEvent::PlaybackError(
                                "malformed response from mpv".to_string(),
                            ))
                            .await;
                        continue;
                    }
                };
                if let Some(request_id) = frame.request_id {
                    if let Some(sender) = pending.lock().await.remove(&request_id) {
                        let response = match frame.error.as_deref() {
                            None | Some("success") => Ok(frame.data.unwrap_or(Value::Null)),
                            Some(error) => Err(AppError::MpvPlayback(error.to_string())),
                        };
                        let _ = sender.send(response);
                    }
                    continue;
                }
                if let Some(event) = event_from_frame(frame)
                    && event_tx.send(event).await.is_err()
                {
                    break;
                }
            }
            Ok(None) => {
                fail_pending(&pending, "mpv IPC disconnected").await;
                let _ = event_tx.send(PlaybackEvent::Shutdown).await;
                break;
            }
            Err(err) => {
                tracing::warn!(?err, "mpv IPC read error");
                fail_pending(&pending, &format!("mpv IPC read error: {err}")).await;
                let _ = event_tx.send(PlaybackEvent::Shutdown).await;
                break;
            }
        }
    }
}

async fn fail_pending(pending: &PendingRequests, message: &str) {
    for (_, sender) in pending.lock().await.drain() {
        let _ = sender.send(Err(AppError::MpvIpc(message.to_string())));
    }
}

/// Translate one raw mpv frame into a typed event.
#[cfg(test)]
fn parse_frame(line: &str) -> Option<PlaybackEvent> {
    let frame: MpvFrame = serde_json::from_str(line).ok()?;
    event_from_frame(frame)
}

fn event_from_frame(frame: MpvFrame) -> Option<PlaybackEvent> {
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
            (Some("speed"), Some(Value::Number(n))) => n.as_f64().map(PlaybackEvent::SpeedChanged),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    use super::*;

    async fn test_client(
        timeout: Duration,
    ) -> (
        MpvIpc,
        mpsc::Receiver<PlaybackEvent>,
        tempfile::TempDir,
        UnixListener,
    ) {
        let temp = tempfile::tempdir().expect("temp dir");
        let socket = temp.path().join("mpv.sock");
        let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
        let (event_tx, event_rx) = mpsc::channel(8);
        let client = MpvIpc::connect_with_timeout(&socket, event_tx, timeout)
            .await
            .expect("connect fake mpv");
        (client, event_rx, temp, listener)
    }

    async fn read_request(lines: &mut tokio::io::Lines<BufReader<UnixStream>>) -> Value {
        let line = lines
            .next_line()
            .await
            .expect("read request")
            .expect("request line");
        serde_json::from_str(&line).expect("request json")
    }

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
    fn parses_speed_change() {
        let event =
            parse_frame(r#"{"event":"property-change","name":"speed","data":1.5}"#).expect("e");
        assert_eq!(event, PlaybackEvent::SpeedChanged(1.5));
    }

    #[test]
    fn ignores_unknown_frames() {
        assert!(parse_frame(r#"{"event":"video-reconfig"}"#).is_none());
        assert!(parse_frame("not json").is_none());
    }

    #[tokio::test]
    async fn command_returns_matching_success_data() {
        let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut lines = BufReader::new(stream).lines();
            let request = read_request(&mut lines).await;
            let request_id = request["request_id"].as_u64().expect("request id");
            let mut stream = lines.into_inner().into_inner();
            stream
                .write_all(
                    format!("{{\"request_id\":{request_id},\"error\":\"success\",\"data\":7}}\n")
                        .as_bytes(),
                )
                .await
                .expect("respond");
        });

        let response = client
            .command(vec![json!("get_property"), json!("volume")])
            .await
            .expect("acknowledged command");
        assert_eq!(response, json!(7));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn command_surfaces_matching_mpv_error() {
        let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut lines = BufReader::new(stream).lines();
            let request = read_request(&mut lines).await;
            let request_id = request["request_id"].as_u64().expect("request id");
            let mut stream = lines.into_inner().into_inner();
            stream
                .write_all(
                    format!("{{\"request_id\":{request_id},\"error\":\"invalid parameter\"}}\n")
                        .as_bytes(),
                )
                .await
                .expect("respond");
        });

        let error = client
            .command(vec![json!("bad")])
            .await
            .expect_err("mpv rejection");
        assert!(matches!(error, AppError::MpvPlayback(message) if message == "invalid parameter"));
    }

    #[tokio::test]
    async fn reordered_responses_reach_their_own_request() {
        let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut lines = BufReader::new(stream).lines();
            let first = read_request(&mut lines).await;
            let second = read_request(&mut lines).await;
            let first_id = first["request_id"].as_u64().expect("first id");
            let second_id = second["request_id"].as_u64().expect("second id");
            let mut stream = lines.into_inner().into_inner();
            stream
                .write_all(
                    format!(
                        "{{\"request_id\":{second_id},\"error\":\"success\",\"data\":\"second\"}}\n{{\"request_id\":{first_id},\"error\":\"success\",\"data\":\"first\"}}\n"
                    )
                    .as_bytes(),
                )
                .await
                .expect("respond");
        });

        let first_client = client.clone();
        let second_client = client.clone();
        let (first, second) = tokio::join!(
            first_client.command(vec![json!("first")]),
            second_client.command(vec![json!("second")])
        );
        assert_eq!(first.expect("first response"), json!("first"));
        assert_eq!(second.expect("second response"), json!("second"));
    }

    #[tokio::test]
    async fn command_times_out_without_a_response() {
        let (client, _events, _temp, listener) = test_client(Duration::from_millis(20)).await;
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("accept");
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        let error = client
            .command(vec![json!("silent")])
            .await
            .expect_err("timeout");
        assert!(matches!(error, AppError::Timeout(_)));
        server.abort();
    }

    #[tokio::test]
    async fn malformed_frame_fails_pending_command() {
        let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            stream.write_all(b"{bad json}\n").await.expect("write");
        });
        let error = client
            .command(vec![json!("malformed")])
            .await
            .expect_err("protocol error");
        assert!(matches!(error, AppError::MpvIpc(_)));
    }

    #[tokio::test]
    async fn eof_fails_pending_command_and_emits_shutdown() {
        let (client, mut events, _temp, listener) = test_client(Duration::from_secs(1)).await;
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            drop(stream);
        });
        let error = client
            .command(vec![json!("disconnect")])
            .await
            .expect_err("disconnect");
        assert!(matches!(error, AppError::MpvIpc(_)));
        assert_eq!(events.recv().await, Some(PlaybackEvent::Shutdown));
    }
}
