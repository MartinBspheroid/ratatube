//! Unix-socket server: accept loop, per-client framing tasks, and the
//! client registry the daemon loop uses for replies and broadcast.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::BufReader;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Semaphore, TryAcquireError, mpsc};

use crate::protocol::{
    ClientFrame, Command, DaemonFrame, PROTOCOL_VERSION, ReplyResult, read_frame, write_frame,
};

/// Outbound queue depth per client; overflow disconnects that client so a
/// slow reader can never block the daemon.
const CLIENT_QUEUE_FRAMES: usize = 1024;
/// Concurrently served connections. Real use is one attached TUI plus the
/// occasional one-shot CLI call, so this is far past generous; the cap exists
/// only so a misbehaving peer cannot spawn tasks and consume file descriptors
/// without bound.
const MAX_CLIENTS: usize = 64;
/// Refusals in flight once the cap is reached. A refusal costs one descriptor
/// for at most `HANDSHAKE_TIMEOUT`; past this many, further connections are
/// dropped unanswered so a flood cannot become a descriptor leak by way of
/// the polite path.
const MAX_REFUSALS_IN_FLIGHT: usize = 16;
/// How long a fresh connection has to send its `Hello`. Real clients send it
/// immediately after connecting; without a bound, a peer that connects and
/// never speaks holds a task and a descriptor for the daemon's whole life.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);

/// Messages from connection tasks to the daemon loop.
#[derive(Debug)]
pub enum ServerMessage {
    /// A client completed a valid hello; the loop registers the sender and
    /// answers with `Welcome`.
    Connected {
        client_id: u64,
        sender: mpsc::Sender<DaemonFrame>,
    },
    /// A correlated command from a connected client.
    Command {
        client_id: u64,
        id: u64,
        command: Box<Command>,
    },
    /// The client's connection ended.
    Disconnected { client_id: u64 },
}

/// Spawn the accept loop; each connection gets its own framing task, up to
/// `MAX_CLIENTS` of them. The loop itself never awaits a peer, so a stalled
/// or refused connection cannot delay the next accept.
pub fn spawn_accept_loop(listener: UnixListener, server_tx: mpsc::Sender<ServerMessage>) {
    tokio::spawn(async move {
        let slots = Arc::new(Semaphore::new(MAX_CLIENTS));
        let refusal_slots = Arc::new(Semaphore::new(MAX_REFUSALS_IN_FLIGHT));
        let mut next_client: u64 = 1;
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let client_id = next_client;
                    next_client = next_client.wrapping_add(1).max(1);
                    match Arc::clone(&slots).try_acquire_owned() {
                        Ok(slot) => {
                            let server_tx = server_tx.clone();
                            tokio::spawn(async move {
                                serve_client(stream, client_id, server_tx).await;
                                drop(slot);
                            });
                        }
                        Err(TryAcquireError::NoPermits) => {
                            tracing::warn!(
                                client_id,
                                MAX_CLIENTS,
                                "client limit reached; refusing connection"
                            );
                            if let Ok(slot) = Arc::clone(&refusal_slots).try_acquire_owned() {
                                tokio::spawn(async move {
                                    refuse_client(stream).await;
                                    drop(slot);
                                });
                            }
                        }
                        Err(TryAcquireError::Closed) => break,
                    }
                }
                Err(err) => {
                    tracing::warn!(?err, "daemon accept failed");
                    break;
                }
            }
        }
    });
}

/// Read a peer's `Hello` so its client library correlates the answer, then
/// tell it the daemon is full. Bounded by `HANDSHAKE_TIMEOUT` on both halves,
/// so a refusal never outlives the handshake it answers.
async fn refuse_client(stream: UnixStream) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let hello = tokio::time::timeout(HANDSHAKE_TIMEOUT, read_frame::<_, ClientFrame>(&mut reader));
    if !matches!(hello.await, Ok(Ok(Some(ClientFrame::Hello { .. })))) {
        return;
    }
    let refusal = DaemonFrame::Reply {
        id: 0,
        result: ReplyResult::Error(format!(
            "daemon is already serving its maximum of {MAX_CLIENTS} clients"
        )),
    };
    let _ = tokio::time::timeout(HANDSHAKE_TIMEOUT, write_frame(&mut write_half, &refusal)).await;
}

/// Handshake, then pump frames in both directions until either side ends.
async fn serve_client(stream: UnixStream, client_id: u64, server_tx: mpsc::Sender<ServerMessage>) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    // Only the handshake is time-bounded: once a client is connected it may
    // sit idle for hours between commands.
    let hello = tokio::time::timeout(HANDSHAKE_TIMEOUT, read_frame::<_, ClientFrame>(&mut reader));
    match hello.await {
        Ok(Ok(Some(ClientFrame::Hello { protocol }))) if protocol == PROTOCOL_VERSION => {}
        Ok(Ok(Some(ClientFrame::Hello { protocol }))) => {
            let refusal = DaemonFrame::Reply {
                id: 0,
                result: ReplyResult::Error(format!(
                    "protocol {protocol} is not supported; this daemon speaks {PROTOCOL_VERSION}"
                )),
            };
            let _ = write_frame(&mut write_half, &refusal).await;
            return;
        }
        Err(_elapsed) => {
            tracing::warn!(client_id, "client did not send hello in time; dropping");
            return;
        }
        _ => return,
    }
    let (sender, mut outbound) = mpsc::channel::<DaemonFrame>(CLIENT_QUEUE_FRAMES);
    if server_tx
        .send(ServerMessage::Connected { client_id, sender })
        .await
        .is_err()
    {
        return;
    }
    let writer = tokio::spawn(async move {
        while let Some(frame) = outbound.recv().await {
            if write_frame(&mut write_half, &frame).await.is_err() {
                break;
            }
        }
    });
    while let Ok(Some(frame)) = read_frame::<_, ClientFrame>(&mut reader).await {
        match frame {
            ClientFrame::Command { id, command } => {
                if server_tx
                    .send(ServerMessage::Command {
                        client_id,
                        id,
                        command,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
            // A second hello is a protocol error; drop the connection.
            ClientFrame::Hello { .. } => break,
        }
    }
    let _ = server_tx
        .send(ServerMessage::Disconnected { client_id })
        .await;
    writer.abort();
}

/// Connected clients, keyed by id. Sends never block: a full queue drops
/// the client instead of stalling the daemon loop.
#[derive(Default)]
pub struct Clients {
    senders: HashMap<u64, mpsc::Sender<DaemonFrame>>,
}

impl Clients {
    pub fn insert(&mut self, client_id: u64, sender: mpsc::Sender<DaemonFrame>) {
        self.senders.insert(client_id, sender);
    }

    pub fn remove(&mut self, client_id: u64) {
        self.senders.remove(&client_id);
    }

    /// Send one frame to one client; a full or closed queue drops them.
    pub fn send_to(&mut self, client_id: u64, frame: DaemonFrame) {
        if let Some(sender) = self.senders.get(&client_id)
            && sender.try_send(frame).is_err()
        {
            tracing::warn!(client_id, "client outbound queue overflow; dropping");
            self.senders.remove(&client_id);
        }
    }

    /// Broadcast one frame to every client, dropping any that cannot keep up.
    pub fn broadcast(&mut self, frame: &DaemonFrame) {
        self.senders
            .retain(|client_id, sender| match sender.try_send(frame.clone()) {
                Ok(()) => true,
                Err(_) => {
                    tracing::warn!(client_id, "client outbound queue overflow; dropping");
                    false
                }
            });
    }
}
