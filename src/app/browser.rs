//! Validated operating-system browser dispatch.

/// Open an allow-listed YouTube URL through the platform browser command.
pub(super) fn open_browser(url: &str) -> std::io::Result<()> {
    if !is_allowed_browser_url(url) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only HTTPS YouTube video URLs are allowed",
        ));
    }
    browser_command(url).spawn().map(|_| ())
}

/// Return whether a URL is a validated HTTPS YouTube video URL.
pub(super) fn is_allowed_browser_url(url: &str) -> bool {
    crate::platform::is_safe_youtube_video_url(url)
}

#[cfg(target_os = "macos")]
fn browser_command(url: &str) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    command.arg(url);
    command
}

#[cfg(target_os = "linux")]
fn browser_command(url: &str) -> std::process::Command {
    let mut command = std::process::Command::new("xdg-open");
    command.arg(url);
    command
}

#[cfg(target_os = "windows")]
fn browser_command(url: &str) -> std::process::Command {
    let mut command = std::process::Command::new("rundll32");
    command.args(["url.dll,FileProtocolHandler", url]);
    command
}
