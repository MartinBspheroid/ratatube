use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

use ratatube_domain::error::{AppError, Result};

const METADATA_TIMEOUT: Duration = Duration::from_secs(60);

/// Run yt-dlp with bounded output capture and a metadata timeout.
pub(super) async fn run(
    binary: &str,
    args: &[&str],
    stdout_limit: usize,
    stderr_limit: usize,
) -> Result<String> {
    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::MissingDependency(binary.to_string())
        } else {
            AppError::Process {
                command: binary.to_string(),
                message: error.to_string(),
            }
        }
    })?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let operation = async {
        let (stdout, stderr) = tokio::try_join!(
            read_limited(stdout, stdout_limit, "yt-dlp stdout"),
            read_limited(stderr, stderr_limit, "yt-dlp stderr")
        )?;
        let status = child.wait().await?;
        Ok::<_, AppError>((status, stdout, stderr))
    };
    match tokio::time::timeout(METADATA_TIMEOUT, operation).await {
        Ok(Ok((status, stdout, _))) if status.success() => {
            Ok(String::from_utf8_lossy(&stdout).into_owned())
        }
        Ok(Ok((_, _, stderr))) => Err(AppError::YtDlp(redact_urls(
            String::from_utf8_lossy(&stderr).trim(),
        ))),
        Ok(Err(error)) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(error)
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(AppError::Timeout(format!(
                "yt-dlp exceeded {}s",
                METADATA_TIMEOUT.as_secs()
            )))
        }
    }
}

async fn read_limited(
    mut reader: impl AsyncRead + Unpin,
    limit: usize,
    resource: &str,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            return Ok(bytes);
        }
        if bytes.len() + read > limit {
            return Err(AppError::ResourceLimit {
                resource: resource.to_string(),
                limit,
            });
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}

fn redact_urls(message: &str) -> String {
    message
        .split_whitespace()
        .map(|part| {
            if part.starts_with("http://") || part.starts_with("https://") {
                "[url redacted]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{read_limited, redact_urls};

    #[tokio::test]
    async fn bounded_reader_accepts_exact_limit_and_rejects_one_byte_over() {
        let exact = read_limited(&b"1234"[..], 4, "test")
            .await
            .expect("exact limit");
        assert_eq!(exact, b"1234");
        assert!(read_limited(&b"12345"[..], 4, "test").await.is_err());
    }

    #[test]
    fn stderr_redaction_removes_signed_urls() {
        let message = redact_urls("failed https://googlevideo.example/x?sig=SECRET retry");
        assert_eq!(message, "failed [url redacted] retry");
        assert!(!message.contains("SECRET"));
    }
}
