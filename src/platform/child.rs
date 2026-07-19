//! Bounded asynchronous child-process lifecycle shared by platform adapters.

use std::ffi::OsString;
use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, Command};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, Result};

const MAX_STDERR_BYTES: usize = 8 * 1024;

/// One direct executable invocation with optional bytes written to stdin.
pub(crate) struct ChildRequest {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub stdin: Option<Vec<u8>>,
    pub label: &'static str,
}

/// Run a child before `deadline`, reaping it on every started-process path.
pub(crate) async fn run_before(
    deadline: Instant,
    request: ChildRequest,
    cancellation: &CancellationToken,
) -> Result<()> {
    let command_name = request.program.display().to_string();
    let mut command = Command::new(&request.program);
    command
        .args(request.args)
        .stdin(if request.stdin.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take();
    let stderr = child.stderr.take();
    let outcome = {
        let lifecycle = async {
            if let Some(payload) = request.stdin {
                let mut pipe = stdin.take().ok_or_else(|| AppError::Process {
                    command: command_name.clone(),
                    message: "child stdin was unavailable".to_string(),
                })?;
                pipe.write_all(&payload).await?;
                pipe.shutdown().await?;
            }
            drop(stdin);
            let (status, stderr) = tokio::join!(child.wait(), read_stderr(stderr));
            Ok::<_, AppError>((status?, stderr?))
        };
        tokio::pin!(lifecycle);
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            outcome = tokio::time::timeout_at(deadline, &mut lifecycle) => Some(outcome),
        }
    };
    match outcome {
        Some(Ok(Ok((status, _stderr)))) if status.success() => Ok(()),
        Some(Ok(Ok((status, stderr)))) => {
            let detail = String::from_utf8_lossy(&stderr);
            let detail = detail.trim();
            let message = if detail.is_empty() {
                format!("exit status {status}")
            } else {
                format!("exit status {status}: {detail}")
            };
            Err(AppError::Process {
                command: command_name,
                message,
            })
        }
        Some(Ok(Err(error))) => {
            terminate_and_reap(&mut child).await;
            Err(error)
        }
        Some(Err(_)) => {
            terminate_and_reap(&mut child).await;
            Err(AppError::Timeout(format!(
                "{} command exceeded its deadline",
                request.label
            )))
        }
        None => {
            terminate_and_reap(&mut child).await;
            Err(AppError::Process {
                command: command_name,
                message: "cancelled".to_string(),
            })
        }
    }
}

async fn read_stderr(mut stderr: Option<ChildStderr>) -> std::io::Result<Vec<u8>> {
    let Some(stderr) = stderr.as_mut() else {
        return Ok(Vec::new());
    };
    let mut captured = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stderr.read(&mut chunk).await?;
        if read == 0 {
            return Ok(captured);
        }
        let remaining = MAX_STDERR_BYTES.saturating_sub(captured.len());
        captured.extend_from_slice(&chunk[..read.min(remaining)]);
    }
}

async fn terminate_and_reap(child: &mut Child) {
    if !matches!(child.try_wait(), Ok(Some(_))) {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

#[cfg(all(test, unix))]
mod tests;
