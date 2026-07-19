//! Validated operating-system browser dispatch.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::time::Instant;

use crate::error::Result;
use crate::platform::child::{ChildRequest, run_before};

const BROWSER_TIMEOUT: Duration = Duration::from_secs(2);

/// Open an allow-listed YouTube URL through the platform browser command.
pub(super) async fn open_browser(url: &str) -> Result<()> {
    let (program, args) = browser_command();
    open_browser_with_command(url, &program, args, BROWSER_TIMEOUT).await
}

pub(super) async fn open_browser_with_command<I, S>(
    url: &str,
    program: &Path,
    args: I,
    timeout: Duration,
) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let deadline = Instant::now() + timeout;
    crate::platform::validate_youtube_video_url(url)?;
    let mut args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect::<Vec<_>>();
    args.push(OsString::from(url));
    run_before(
        deadline,
        ChildRequest {
            program: program.to_path_buf(),
            args,
            stdin: None,
            label: "browser",
        },
    )
    .await
}

/// Return whether a URL is a validated HTTPS YouTube video URL.
#[cfg(test)]
pub(super) fn is_allowed_browser_url(url: &str) -> bool {
    crate::platform::is_safe_youtube_video_url(url)
}

#[cfg(target_os = "macos")]
fn browser_command() -> (PathBuf, Vec<OsString>) {
    (PathBuf::from("open"), Vec::new())
}

#[cfg(target_os = "linux")]
fn browser_command() -> (PathBuf, Vec<OsString>) {
    (PathBuf::from("xdg-open"), Vec::new())
}

#[cfg(target_os = "windows")]
fn browser_command() -> (PathBuf, Vec<OsString>) {
    (
        PathBuf::from("rundll32"),
        vec![OsString::from("url.dll,FileProtocolHandler")],
    )
}
