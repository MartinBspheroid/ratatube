//! mpv JSON IPC client.
//!
//! mpv speaks newline-delimited JSON over a unix domain socket (PRD 10.3).
//! One owner task reads events; commands carry a monotonically increasing
//! `request_id` so responses can be correlated (PRD section 15).

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::net::unix::OwnedWriteHalf;
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::error::{AppError, Result};
use crate::playback::events::PlaybackEvent;

mod frame;
mod reader;
#[cfg(test)]
mod tests;

use reader::{PendingRequests, read_events};

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
        let pending = PendingRequests::default();
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
        self.pending.insert(request_id, response_tx).await;
        let frame = json!({ "command": args, "request_id": request_id });
        let mut line = serde_json::to_vec(&frame)?;
        line.push(b'\n');
        if let Err(err) = self.writer.lock().await.write_all(&line).await {
            self.pending.remove(&request_id).await;
            return Err(AppError::MpvIpc(format!("write: {err}")));
        }
        match tokio::time::timeout(self.command_timeout, response_rx).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => Err(AppError::MpvIpc(
                "response router closed before acknowledgement".to_string(),
            )),
            Err(_) => {
                self.pending.remove(&request_id).await;
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
