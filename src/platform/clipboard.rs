//! Clipboard adapter for canonical YouTube video URLs.

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::{AppError, Result};

const CLIPBOARD_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Copy one validated YouTube video URL through the native clipboard command.
pub fn copy_url(url: &str) -> Result<()> {
    validate_url(url)?;
    let (program, args) = native_clipboard_command()?;
    copy_url_with_command(url, &program, args, CLIPBOARD_TIMEOUT)
}

fn copy_url_with_command<I, S>(url: &str, program: &Path, args: I, timeout: Duration) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    validate_url(url)?;
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let write_result = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Process {
            command: program.display().to_string(),
            message: "clipboard stdin was unavailable".to_string(),
        })?
        .write_all(url.as_bytes());
    if let Err(error) = write_result {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error.into());
    }

    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            let detail = stderr.trim();
            let message = if detail.is_empty() {
                format!("exit status {status}")
            } else {
                format!("exit status {status}: {detail}")
            };
            return Err(AppError::Process {
                command: program.display().to_string(),
                message,
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::Timeout(format!(
                "clipboard command exceeded {} ms",
                timeout.as_millis()
            )));
        }
        std::thread::sleep(POLL_INTERVAL.min(timeout));
    }
}

fn validate_url(url: &str) -> Result<()> {
    if crate::platform::is_safe_youtube_video_url(url) {
        Ok(())
    } else {
        Err(AppError::InvalidUrl(
            "expected an HTTPS YouTube video URL".to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn native_clipboard_command() -> Result<(PathBuf, Vec<&'static str>)> {
    Ok((PathBuf::from("pbcopy"), Vec::new()))
}

#[cfg(target_os = "linux")]
fn native_clipboard_command() -> Result<(PathBuf, Vec<&'static str>)> {
    if let Ok(program) = which::which("wl-copy") {
        return Ok((program, Vec::new()));
    }
    if let Ok(program) = which::which("xclip") {
        return Ok((program, vec!["-selection", "clipboard"]));
    }
    Err(AppError::MissingDependency("wl-copy or xclip".to_string()))
}

#[cfg(target_os = "windows")]
fn native_clipboard_command() -> Result<(PathBuf, Vec<&'static str>)> {
    Ok((PathBuf::from("clip.exe"), Vec::new()))
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::error::AppError;

    const VIDEO_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";

    fn fake_clipboard(temp: &tempfile::TempDir, body: &str) -> std::path::PathBuf {
        let path = temp.path().join("fake-clipboard");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake executable");
        let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("make fake executable");
        path
    }

    #[test]
    fn clipboard_writes_valid_url_to_stdin_byte_for_byte_after_zero_exit() {
        let temp = tempfile::tempdir().expect("tempdir");
        let output = temp.path().join("stdin.txt");
        let command = fake_clipboard(&temp, "dd of=\"$1\" 2>/dev/null");

        copy_url_with_command(VIDEO_URL, &command, [&output], Duration::from_secs(1))
            .expect("clipboard success");

        assert_eq!(
            fs::read(&output).expect("recorded stdin"),
            VIDEO_URL.as_bytes()
        );
    }

    #[test]
    fn clipboard_rejects_malformed_or_non_https_urls_before_spawning() {
        let missing = Path::new("/definitely/missing/fake-clipboard");
        for url in [
            "http://www.youtube.com/watch?v=video",
            "https://example.com/watch?v=video",
            "https://youtube.com@example.com/watch?v=video",
            "https://www.youtube.com/channel/not-a-video",
            "not a url",
        ] {
            let error = copy_url_with_command(
                url,
                missing,
                std::iter::empty::<&str>(),
                Duration::from_millis(50),
            )
            .expect_err("unsafe URL must fail");
            assert!(matches!(error, AppError::InvalidUrl(_)), "{url}: {error}");
        }
    }

    #[test]
    fn clipboard_reports_non_zero_exit() {
        let temp = tempfile::tempdir().expect("tempdir");
        let command = fake_clipboard(&temp, "dd of=/dev/null 2>/dev/null; exit 17");

        let error = copy_url_with_command(
            VIDEO_URL,
            &command,
            std::iter::empty::<&str>(),
            Duration::from_secs(1),
        )
        .expect_err("non-zero exit must fail");

        assert!(matches!(error, AppError::Process { .. }));
        assert!(error.to_string().contains("17"));
    }

    #[test]
    fn clipboard_kills_a_command_that_exceeds_the_timeout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let command = fake_clipboard(&temp, "while :; do :; done");

        let error = copy_url_with_command(
            VIDEO_URL,
            &command,
            std::iter::empty::<&str>(),
            Duration::from_millis(25),
        )
        .expect_err("timeout must fail");

        assert!(matches!(error, AppError::Timeout(_)));
    }
}
