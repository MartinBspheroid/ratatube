//! Clipboard adapter for canonical YouTube video URLs.

#[cfg(test)]
use std::ffi::OsStr;
use std::ffi::OsString;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, Result};
use crate::platform::child::{ChildRequest, run_before};

const CLIPBOARD_TIMEOUT: Duration = Duration::from_secs(2);
type ClipboardCommand = (PathBuf, Vec<OsString>);

/// Copy one validated YouTube video URL through the native clipboard command.
pub async fn copy_url(url: &str, cancellation: &CancellationToken) -> Result<()> {
    let deadline = Instant::now() + CLIPBOARD_TIMEOUT;
    crate::platform::validate_youtube_video_url(url)?;
    copy_url_before(url, native_clipboard_commands()?, deadline, cancellation).await
}

#[cfg(test)]
async fn copy_url_with_command<I, S>(
    url: &str,
    program: &Path,
    args: I,
    timeout: Duration,
) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let cancellation = CancellationToken::new();
    crate::platform::validate_youtube_video_url(url)?;
    let commands = vec![(
        program.to_path_buf(),
        args.into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect(),
    )];
    copy_url_before(url, commands, Instant::now() + timeout, &cancellation).await
}

#[cfg(test)]
async fn copy_url_with_commands(
    url: &str,
    commands: Vec<ClipboardCommand>,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let cancellation = CancellationToken::new();
    crate::platform::validate_youtube_video_url(url)?;
    copy_url_before(url, commands, deadline, &cancellation).await
}

async fn copy_url_before(
    url: &str,
    commands: Vec<ClipboardCommand>,
    deadline: Instant,
    cancellation: &CancellationToken,
) -> Result<()> {
    let mut last_error = None;
    for (program, args) in commands {
        if Instant::now() >= deadline {
            return Err(AppError::Timeout(
                "clipboard command exceeded its deadline".to_string(),
            ));
        }
        let request = ChildRequest {
            program,
            args,
            stdin: Some(url.as_bytes().to_vec()),
            label: "clipboard",
        };
        match run_before(deadline, request, cancellation).await {
            Ok(()) => return Ok(()),
            Err(error) if cancellation.is_cancelled() => return Err(error),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        AppError::MissingDependency("clipboard adapter for the active display session".to_string())
    }))
}

#[cfg(target_os = "macos")]
fn native_clipboard_commands() -> Result<Vec<ClipboardCommand>> {
    Ok(vec![(PathBuf::from("pbcopy"), Vec::new())])
}

#[cfg(target_os = "linux")]
fn native_clipboard_commands() -> Result<Vec<ClipboardCommand>> {
    let wayland = std::env::var("WAYLAND_DISPLAY").ok();
    let display = std::env::var("DISPLAY").ok();
    let mut commands = Vec::new();
    for adapter in linux_candidate_order(wayland.as_deref(), display.as_deref()) {
        match adapter {
            LinuxClipboardAdapter::Wayland => {
                if let Ok(program) = which::which("wl-copy") {
                    commands.push((program, Vec::new()));
                }
            }
            LinuxClipboardAdapter::X11 => {
                if let Ok(program) = which::which("xclip") {
                    commands.push((
                        program,
                        vec![OsString::from("-selection"), OsString::from("clipboard")],
                    ));
                }
            }
        }
    }
    if commands.is_empty() {
        Err(AppError::MissingDependency(
            "wl-copy for Wayland or xclip for X11".to_string(),
        ))
    } else {
        Ok(commands)
    }
}

#[cfg(target_os = "windows")]
fn native_clipboard_commands() -> Result<Vec<ClipboardCommand>> {
    Ok(vec![(PathBuf::from("clip.exe"), Vec::new())])
}

#[cfg(any(target_os = "linux", all(test, unix)))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinuxClipboardAdapter {
    Wayland,
    X11,
}

#[cfg(any(target_os = "linux", all(test, unix)))]
fn linux_candidate_order(
    wayland_display: Option<&str>,
    display: Option<&str>,
) -> Vec<LinuxClipboardAdapter> {
    let mut adapters = Vec::new();
    if wayland_display.is_some_and(|value| !value.is_empty()) {
        adapters.push(LinuxClipboardAdapter::Wayland);
    }
    if display.is_some_and(|value| !value.is_empty()) {
        adapters.push(LinuxClipboardAdapter::X11);
    }
    adapters
}

#[cfg(all(test, unix))]
mod tests;
