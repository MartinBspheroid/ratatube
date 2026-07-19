use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::Duration;

use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use super::{ChildRequest, run_before};

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
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\nprintf '%s' \"$$\" > '{}'\nexec sleep 30\n",
            pid_path.display()
        ),
    )
    .expect("write fake executable");
    let mut permissions = fs::metadata(&executable).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).expect("make executable");
    let cancellation = CancellationToken::new();
    let cancel = cancellation.clone();
    tokio::spawn(async move {
        while !pid_path.exists() {
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
