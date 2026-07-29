use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::*;
use ratatube_domain::error::AppError;

const VIDEO_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_TEST_BUDGET: Duration = Duration::from_secs(3);

fn fake_clipboard(temp: &tempfile::TempDir, name: &str, body: &str) -> PathBuf {
    let path = temp.path().join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake executable");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

fn read_pid(path: &Path) -> u32 {
    fs::read_to_string(path)
        .expect("pid file")
        .trim()
        .parse()
        .expect("numeric pid")
}

fn assert_reaped(pid: u32) {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .expect("inspect process");
    assert!(
        !output.status.success() || output.stdout.is_empty(),
        "pid {pid} still exists with state {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[tokio::test]
async fn clipboard_writes_valid_url_to_stdin_byte_for_byte_after_zero_exit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = temp.path().join("stdin.txt");
    let command = fake_clipboard(&temp, "success", "dd of=\"$1\" 2>/dev/null");

    copy_url_with_command(VIDEO_URL, &command, [&output], COMPLETION_TIMEOUT)
        .await
        .expect("clipboard success");

    assert_eq!(
        fs::read(&output).expect("recorded stdin"),
        VIDEO_URL.as_bytes()
    );
}

#[tokio::test]
async fn clipboard_rejects_unsafe_and_oversized_urls_before_spawning() {
    let missing = Path::new("/definitely/missing/fake-clipboard");
    for url in [
        "http://www.youtube.com/watch?v=video".to_string(),
        "https://example.com/watch?v=video".to_string(),
        "https://youtube.com@example.com/watch?v=video".to_string(),
        "https://www.youtube.com/channel/not-a-video".to_string(),
        "not a url".to_string(),
        format!(
            "https://www.youtube.com/watch?v={}",
            "a".repeat(crate::platform::MAX_EXTERNAL_URL_BYTES)
        ),
    ] {
        let error = copy_url_with_command(
            &url,
            missing,
            std::iter::empty::<&str>(),
            Duration::from_millis(50),
        )
        .await
        .expect_err("unsafe URL must fail");
        assert!(
            matches!(
                error,
                AppError::InvalidUrl(_) | AppError::ResourceLimit { .. }
            ),
            "{url}: {error}"
        );
    }
}

#[tokio::test]
async fn clipboard_nonzero_exit_is_reported_after_child_is_reaped() {
    let temp = tempfile::tempdir().expect("tempdir");
    let pid_path = temp.path().join("pid");
    let command = fake_clipboard(
        &temp,
        "nonzero",
        "printf '%s' \"$$\" > \"$1\"; dd of=/dev/null 2>/dev/null; echo denied >&2; exit 17",
    );

    let error = copy_url_with_command(VIDEO_URL, &command, [&pid_path], COMPLETION_TIMEOUT)
        .await
        .expect_err("non-zero exit must fail");

    assert!(matches!(error, AppError::Process { .. }));
    assert!(error.to_string().contains("17"));
    assert!(error.to_string().contains("denied"));
    assert_reaped(read_pid(&pid_path));
}

#[tokio::test]
async fn clipboard_timeout_kills_and_reaps_recorded_pid() {
    let temp = tempfile::tempdir().expect("tempdir");
    let pid_path = temp.path().join("pid");
    let command = fake_clipboard(
        &temp,
        "timeout",
        "printf '%s' \"$$\" > \"$1\"; exec sleep 30",
    );

    let error = copy_url_with_command(VIDEO_URL, &command, [&pid_path], TIMEOUT_TEST_BUDGET)
        .await
        .expect_err("timeout must fail");

    assert!(matches!(error, AppError::Timeout(_)));
    assert_reaped(read_pid(&pid_path));
}

#[tokio::test]
async fn clipboard_falls_back_after_session_incompatible_candidate() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = temp.path().join("stdin.txt");
    let incompatible = fake_clipboard(&temp, "incompatible", "exit 1");
    let fallback = fake_clipboard(&temp, "fallback", "dd of=\"$1\" 2>/dev/null");
    let commands = vec![
        (incompatible, Vec::new()),
        (fallback, vec![OsString::from(output.as_os_str())]),
    ];

    copy_url_with_commands(VIDEO_URL, commands, COMPLETION_TIMEOUT)
        .await
        .expect("fallback succeeds");

    assert_eq!(
        fs::read(output).expect("fallback stdin"),
        VIDEO_URL.as_bytes()
    );
}

#[test]
fn linux_candidate_order_respects_active_display_sessions() {
    use LinuxClipboardAdapter::{Wayland, X11};

    assert_eq!(
        linux_candidate_order(Some("wayland-0"), Some(":0")),
        vec![Wayland, X11]
    );
    assert_eq!(
        linux_candidate_order(Some("wayland-0"), None),
        vec![Wayland]
    );
    assert_eq!(linux_candidate_order(None, Some(":0")), vec![X11]);
    assert!(linux_candidate_order(None, None).is_empty());
    assert!(linux_candidate_order(Some(""), Some("")).is_empty());
}
