//! Validated operating-system browser dispatch.

pub(super) fn open_browser(url: &str) -> std::io::Result<()> {
    if !is_allowed_browser_url(url) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only YouTube HTTP(S) URLs are allowed",
        ));
    }
    browser_command(url).spawn().map(|_| ())
}

pub(super) fn is_allowed_browser_url(url: &str) -> bool {
    let Some(remainder) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let host = authority.split(':').next().unwrap_or_default();
    matches!(
        host,
        "youtube.com" | "www.youtube.com" | "music.youtube.com" | "youtu.be"
    )
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
