use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::unix::OwnedReadHalf;
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::error::{AppError, Result};
use crate::playback::events::PlaybackEvent;
use crate::playback::ipc::frame::{MpvFrame, event_from_frame};

type ResponseSender = oneshot::Sender<Result<Value>>;

#[derive(Clone, Default)]
pub(super) struct PendingRequests(Arc<Mutex<HashMap<u64, ResponseSender>>>);

impl PendingRequests {
    pub(super) async fn insert(&self, request_id: u64, sender: ResponseSender) {
        self.0.lock().await.insert(request_id, sender);
    }

    pub(super) async fn remove(&self, request_id: &u64) -> Option<ResponseSender> {
        self.0.lock().await.remove(request_id)
    }

    async fn fail_all(&self, message: &str) {
        for (_, sender) in self.0.lock().await.drain() {
            let _ = sender.send(Err(AppError::MpvIpc(message.to_string())));
        }
    }
}

/// Own the read half and route newline-delimited mpv frames.
pub(super) async fn read_events(
    reader: OwnedReadHalf,
    event_tx: mpsc::Sender<PlaybackEvent>,
    pending: PendingRequests,
) {
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if !route_line(&line, &event_tx, &pending).await {
                    break;
                }
            }
            Ok(None) => {
                pending.fail_all("mpv IPC disconnected").await;
                let _ = event_tx.send(PlaybackEvent::Shutdown).await;
                break;
            }
            Err(error) => {
                tracing::warn!(?error, "mpv IPC read error");
                pending
                    .fail_all(&format!("mpv IPC read error: {error}"))
                    .await;
                let _ = event_tx.send(PlaybackEvent::Shutdown).await;
                break;
            }
        }
    }
}

async fn route_line(
    line: &str,
    event_tx: &mpsc::Sender<PlaybackEvent>,
    pending: &PendingRequests,
) -> bool {
    let frame = match serde_json::from_str::<MpvFrame>(line) {
        Ok(frame) => frame,
        Err(error) => {
            pending
                .fail_all(&format!("malformed mpv frame: {error}"))
                .await;
            let _ = event_tx
                .send(PlaybackEvent::PlaybackError(
                    "malformed response from mpv".to_string(),
                ))
                .await;
            return true;
        }
    };
    if let Some(request_id) = frame.request_id {
        if let Some(sender) = pending.remove(&request_id).await {
            let response = match frame.error.as_deref() {
                None | Some("success") => Ok(frame.data.unwrap_or(Value::Null)),
                Some(error) => Err(AppError::MpvPlayback(error.to_string())),
            };
            let _ = sender.send(response);
        }
        return true;
    }
    if let Some(event) = event_from_frame(frame) {
        return event_tx.send(event).await.is_ok();
    }
    true
}
