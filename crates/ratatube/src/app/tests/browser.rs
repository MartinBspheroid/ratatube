#[cfg(unix)]
use std::fs;
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::process::Command;
use std::time::Duration;

#[cfg(unix)]
use super::fake_executable::write_fake_executable;
use crate::app::action::{Action, ExternalCommandKind, ExternalCommandTarget, NavigationAction};
use crate::app::browser::{is_allowed_browser_url, open_browser_with_command};
use crate::app::state::View;
use crate::app::tests::test_app;
use crate::error::AppError;
use crate::media::Track;

const VIDEO_URL: &str = "https://www.youtube.com/watch?v=safe";
#[cfg(unix)]
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(unix)]
const TIMEOUT_TEST_BUDGET: Duration = Duration::from_secs(3);

#[cfg(unix)]
fn fake_opener(temp: &tempfile::TempDir, name: &str, body: &str) -> PathBuf {
    let path = temp.path().join(name);
    write_fake_executable(&path, "/bin/sh", body);
    path
}

#[cfg(unix)]
fn read_pid(path: &Path) -> u32 {
    fs::read_to_string(path)
        .expect("pid file")
        .trim()
        .parse()
        .expect("numeric pid")
}

#[cfg(unix)]
fn assert_reaped(pid: u32) {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .expect("inspect process");
    assert!(!output.status.success() || output.stdout.is_empty());
}

#[test]
fn browser_dispatch_rejects_unsafe_schemes_and_hosts() {
    assert!(is_allowed_browser_url(
        "https://www.youtube.com/watch?v=safe"
    ));
    assert!(is_allowed_browser_url("https://youtu.be/safe"));
    assert!(!is_allowed_browser_url(
        "http://www.youtube.com/watch?v=unsafe"
    ));
    assert!(!is_allowed_browser_url(
        "https://www.youtube.com/channel/not-a-video"
    ));
    assert!(!is_allowed_browser_url("file:///etc/passwd"));
    assert!(!is_allowed_browser_url(
        "https://youtube.com@example.com/attack"
    ));
    assert!(!is_allowed_browser_url("https://example.com/watch?v=no"));
}

#[cfg(unix)]
#[tokio::test]
async fn browser_reports_success_only_after_zero_exit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = temp.path().join("url");
    let opener = fake_opener(&temp, "success", "printf '%s' \"$2\" > \"$1\"; exit 0");

    open_browser_with_command(VIDEO_URL, &opener, [&output], COMPLETION_TIMEOUT)
        .await
        .expect("zero exit");

    assert_eq!(fs::read_to_string(output).expect("opened URL"), VIDEO_URL);
}

#[cfg(unix)]
#[tokio::test]
async fn browser_nonzero_exit_is_reaped_and_reported() {
    let temp = tempfile::tempdir().expect("tempdir");
    let pid_path = temp.path().join("pid");
    let opener = fake_opener(
        &temp,
        "nonzero",
        "printf '%s' \"$$\" > \"$1\"; echo blocked >&2; exit 23",
    );

    let error = open_browser_with_command(VIDEO_URL, &opener, [&pid_path], COMPLETION_TIMEOUT)
        .await
        .expect_err("nonzero must fail");

    assert!(matches!(error, AppError::Process { .. }));
    assert!(error.to_string().contains("23"));
    assert!(error.to_string().contains("blocked"));
    assert_reaped(read_pid(&pid_path));
}

#[cfg(unix)]
#[tokio::test]
async fn browser_timeout_kills_and_reaps_recorded_pid() {
    let temp = tempfile::tempdir().expect("tempdir");
    let pid_path = temp.path().join("pid");
    let opener = fake_opener(
        &temp,
        "timeout",
        "printf '%s' \"$$\" > \"$1\"; exec sleep 30",
    );

    let error = open_browser_with_command(VIDEO_URL, &opener, [&pid_path], TIMEOUT_TEST_BUDGET)
        .await
        .expect_err("timeout must fail");

    assert!(matches!(error, AppError::Timeout(_)));
    assert_reaped(read_pid(&pid_path));
}

#[tokio::test]
async fn browser_spawn_failure_is_reported() {
    let error = open_browser_with_command(
        VIDEO_URL,
        Path::new("/definitely/missing/fake-opener"),
        std::iter::empty::<&str>(),
        Duration::from_millis(50),
    )
    .await
    .expect_err("spawn failure");

    assert!(matches!(error, AppError::Io(_)));
}

#[tokio::test]
async fn direct_browser_action_completes_through_background_action() {
    let (_temp, mut app) = test_app();
    let mut track = Track::new("unsafe", "Unsafe", "Artist");
    track.webpage_url = "file:///tmp/not-a-video".to_string();
    app.state.ui.view = View::Search;
    app.state.domain.search = crate::media::search::SearchState::Results {
        query: "unsafe".to_string(),
        tracks: vec![track],
    };
    let (action_tx, mut action_rx) = tokio::sync::mpsc::channel(2);

    app.handle_action(
        Action::Navigation(NavigationAction::OpenInBrowser),
        &action_tx,
    )
    .await;

    assert!(app.state.ui.notification.is_none());
    let completion = tokio::time::timeout(Duration::from_millis(200), action_rx.recv())
        .await
        .expect("background browser completion timed out")
        .expect("completion channel closed");
    assert!(matches!(
        completion,
        Action::Navigation(NavigationAction::ExternalCommandCompleted {
            command: ExternalCommandKind::Browser,
            target: ExternalCommandTarget::Direct,
            result: Err(_),
            ..
        })
    ));
    app.handle_action(completion, &action_tx).await;
    assert!(
        app.state
            .ui
            .notification
            .as_ref()
            .is_some_and(|notification| notification.is_error)
    );
}
