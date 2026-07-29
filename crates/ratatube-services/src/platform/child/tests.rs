use std::fs;
use std::process::Command;
use std::time::Duration;

use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use super::{ChildRequest, run_before};

/// Shared with the sibling unit tests and the `yt_dlp` integration test — see
/// the file's own header for why it is included by path. `duplicate_mod` is
/// expected: `platform::clipboard::tests` loads the same file, and neither test
/// module can see the other's private items.
#[path = "../../../tests/support/fake_executable.rs"]
#[allow(clippy::duplicate_mod)]
mod fake_executable;

fn assert_reaped(pid: u32) {
    let output = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .expect("inspect process");
    assert!(
        !output.status.success() || output.stdout.is_empty(),
        "cancelled pid {pid} was not reaped"
    );
}

#[tokio::test]
async fn cancellation_kills_and_reaps_started_child_before_returning() {
    let temp = tempfile::tempdir().expect("tempdir");
    let pid_path = temp.path().join("pid");
    let executable = temp.path().join("slow-command");
    fake_executable::write_fake_executable(
        &executable,
        "/bin/sh",
        &format!(
            "printf '%s' \"$$\" > '{}'\nexec sleep 30",
            pid_path.display()
        ),
    );
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    tokio::spawn(async move {
        while fs::read_to_string(&pid_path)
            .ok()
            .and_then(|contents| contents.trim().parse::<u32>().ok())
            .is_none()
        {
            tokio::task::yield_now().await;
        }
        cancel.cancel();
    });

    let error = run_before(
        Instant::now() + Duration::from_secs(5),
        ChildRequest {
            program: executable,
            args: Vec::new(),
            stdin: None,
            label: "test",
        },
        &cancellation,
    )
    .await
    .expect_err("cancellation must stop command");

    assert!(error.to_string().contains("cancelled"));
    let pid = fs::read_to_string(temp.path().join("pid"))
        .expect("pid file")
        .trim()
        .parse()
        .expect("numeric pid");
    assert_reaped(pid);
}
