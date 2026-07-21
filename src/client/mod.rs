//! Daemon client: connect, handshake, correlated requests, auto-spawn.

pub mod mirror;

use std::path::Path;
use std::time::Duration;

use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

use crate::error::{AppError, Result};
use crate::persistence::AppPaths;
use crate::protocol::{
    self, ClientFrame, Command, DaemonFrame, PROTOCOL_VERSION, ReplyBody, ReplyResult, Snapshot,
};

/// How long one request may wait for its correlated reply.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Auto-spawn connection budget: retries * delay ≈ 3 s.
const SPAWN_RETRIES: u32 = 20;
const SPAWN_RETRY_DELAY: Duration = Duration::from_millis(150);

/// An attached daemon connection after a completed handshake.
pub struct Connection {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    next_id: u64,
    /// Domain snapshot received with `Welcome`.
    pub snapshot: Snapshot,
}

impl Connection {
    /// Connect to `socket` and complete the hello/welcome handshake.
    pub async fn connect(socket: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket).await.map_err(|err| {
            AppError::Config(format!(
                "daemon is not running at {} ({err})",
                socket.display()
            ))
        })?;
        let (read_half, mut writer) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        protocol::write_frame(
            &mut writer,
            &ClientFrame::Hello {
                protocol: PROTOCOL_VERSION,
            },
        )
        .await
        .map_err(io_error)?;
        let welcome = tokio::time::timeout(REQUEST_TIMEOUT, protocol::read_frame(&mut reader))
            .await
            .map_err(|_| AppError::Config("daemon handshake timed out".to_string()))?
            .map_err(io_error)?;
        match welcome {
            Some(DaemonFrame::Welcome { protocol, snapshot }) if protocol == PROTOCOL_VERSION => {
                Ok(Self {
                    reader,
                    writer,
                    next_id: 1,
                    snapshot: *snapshot,
                })
            }
            Some(DaemonFrame::Reply {
                result: ReplyResult::Error(message),
                ..
            }) => Err(AppError::Config(message)),
            other => Err(AppError::Config(format!(
                "unexpected handshake answer: {other:?}"
            ))),
        }
    }

    /// Send one command and await its correlated reply. Broadcast events
    /// arriving in between are skipped (one-shot clients ignore them).
    pub async fn request(&mut self, command: Command) -> Result<ReplyBody> {
        let id = self.next_id;
        self.next_id += 1;
        protocol::write_frame(
            &mut self.writer,
            &ClientFrame::Command {
                id,
                command: Box::new(command),
            },
        )
        .await
        .map_err(io_error)?;
        tokio::time::timeout(REQUEST_TIMEOUT, self.await_reply(id))
            .await
            .map_err(|_| AppError::Config("daemon did not reply in time".to_string()))?
    }

    async fn await_reply(&mut self, id: u64) -> Result<ReplyBody> {
        loop {
            let frame = protocol::read_frame(&mut self.reader)
                .await
                .map_err(io_error)?;
            match frame {
                Some(DaemonFrame::Reply {
                    id: reply_id,
                    result,
                }) if reply_id == id => {
                    return match result {
                        ReplyResult::Result(body) => Ok(body),
                        ReplyResult::Error(message) => Err(AppError::Config(message)),
                    };
                }
                Some(DaemonFrame::Event { .. }) | Some(DaemonFrame::Reply { .. }) => {}
                Some(DaemonFrame::Welcome { .. }) => {
                    return Err(AppError::Config(
                        "unexpected second welcome from daemon".to_string(),
                    ));
                }
                None => {
                    return Err(AppError::Config(
                        "daemon closed the connection before replying".to_string(),
                    ));
                }
            }
        }
    }
}

/// Non-blocking command writer for the streaming (TUI) mode.
pub struct CommandSender {
    writer: OwnedWriteHalf,
    next_id: u64,
}

impl CommandSender {
    /// Send one command; the reply arrives on the frame stream with the
    /// returned id.
    pub async fn send(&mut self, command: Command) -> Result<u64> {
        let id = self.next_id;
        self.next_id += 1;
        protocol::write_frame(
            &mut self.writer,
            &ClientFrame::Command {
                id,
                command: Box::new(command),
            },
        )
        .await
        .map_err(io_error)?;
        Ok(id)
    }
}

impl Connection {
    /// Split into streaming halves for the TUI: a command sender, the
    /// welcome snapshot, and a receiver fed by a background reader task.
    /// The receiver closing means the daemon connection is gone.
    pub fn into_stream(
        self,
    ) -> (
        CommandSender,
        Snapshot,
        tokio::sync::mpsc::Receiver<DaemonFrame>,
    ) {
        let (frame_tx, frame_rx) = tokio::sync::mpsc::channel::<DaemonFrame>(1024);
        let mut reader = self.reader;
        tokio::spawn(async move {
            while let Ok(Some(frame)) = protocol::read_frame::<_, DaemonFrame>(&mut reader).await {
                if frame_tx.send(frame).await.is_err() {
                    break;
                }
            }
        });
        (
            CommandSender {
                writer: self.writer,
                next_id: self.next_id,
            },
            self.snapshot,
            frame_rx,
        )
    }
}

/// Connect, transparently starting the daemon when the socket is silent.
/// `resume` only matters when a daemon is actually spawned: it resumes the
/// previous session at daemon startup.
pub async fn connect_or_spawn(paths: &AppPaths, resume: bool) -> Result<Connection> {
    let socket = crate::daemon::socket_path(paths);
    if let Ok(connection) = Connection::connect(&socket).await {
        return Ok(connection);
    }
    spawn_daemon(paths, resume)?;
    for _ in 0..SPAWN_RETRIES {
        tokio::time::sleep(SPAWN_RETRY_DELAY).await;
        if let Ok(connection) = Connection::connect(&socket).await {
            return Ok(connection);
        }
    }
    Err(AppError::Config(format!(
        "daemon did not start within {:.1}s; check {} for details",
        (SPAWN_RETRIES * SPAWN_RETRY_DELAY.as_millis() as u32) as f64 / 1000.0,
        paths.data_dir.join("ytm-tui.log").display()
    )))
}

/// Start `current_exe() daemon` detached in its own process group; its
/// stdio goes to the log via the daemon's own tracing setup.
fn spawn_daemon(paths: &AppPaths, resume: bool) -> Result<()> {
    let exe = std::env::current_exe()
        .map_err(|err| AppError::Config(format!("cannot locate own executable: {err}")))?;
    let mut command = std::process::Command::new(exe);
    command.arg("daemon").arg("--data-dir").arg(&paths.data_dir);
    if resume {
        command.arg("--resume");
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|err| AppError::Config(format!("could not start the daemon: {err}")))
}

fn io_error(err: std::io::Error) -> AppError {
    AppError::Config(format!("daemon connection failed: {err}"))
}
