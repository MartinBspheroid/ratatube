//! Black-box diagnostics tests. Every invocation uses a disposable data root.

use std::process::Command;

fn doctor(data_dir: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ratatube"))
        .args([
            "--data-dir",
            data_dir.to_str().expect("utf8 path"),
            "doctor",
        ])
        .output()
        .expect("run doctor")
}

#[test]
fn play_cli_declares_and_enforces_a_required_query() {
    let output = Command::new(env!("CARGO_BIN_EXE_ratatube"))
        .arg("play")
        .output()
        .expect("run play without query");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("<QUERY>..."), "stderr:\n{stderr}");
}

#[test]
fn doctor_does_not_create_an_absent_data_root_or_log() {
    let temp = tempfile::tempdir().expect("temp dir");
    let data_dir = temp.path().join("not-created");

    let output = doctor(&data_dir);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("defaults in use"), "stdout:\n{stdout}");
    assert!(stdout.contains("not created yet"), "stdout:\n{stdout}");
    assert!(!data_dir.exists(), "doctor must remain read-only");
}

#[test]
fn doctor_preserves_a_seeded_log_byte_for_byte() {
    let temp = tempfile::tempdir().expect("temp dir");
    let log = temp.path().join("ratatube.log");
    std::fs::write(&log, b"evidence from the prior crash\n").expect("seed log");

    let _ = doctor(temp.path());

    assert_eq!(
        std::fs::read(&log).expect("read log"),
        b"evidence from the prior crash\n"
    );
    assert!(!temp.path().join("ratatube.log.1").exists());
}

#[test]
fn doctor_reports_malformed_config_without_rewriting_or_backing_it_up() {
    let temp = tempfile::tempdir().expect("temp dir");
    let config = temp.path().join("config.json");
    std::fs::write(&config, b"{malformed").expect("seed malformed config");

    let output = doctor(temp.path());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("FAIL config"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("doctor made no changes"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        std::fs::read(&config).expect("config remains"),
        b"{malformed"
    );
    assert!(!temp.path().join("config.json.bak").exists());
}

#[test]
fn doctor_continues_after_config_error_and_reports_missing_dependencies() {
    let temp = tempfile::tempdir().expect("temp dir");
    let config = serde_json::json!({
        "paths": {
            "mpv": "definitely-missing-mpv-for-doctor-test",
            "ytDlp": "definitely-missing-ytdlp-for-doctor-test"
        }
    });
    std::fs::write(
        temp.path().join("config.json"),
        serde_json::to_vec(&config).expect("serialize config"),
    )
    .expect("write config");

    let output = doctor(temp.path());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success());
    assert!(stdout.contains("FAIL mpv"), "stdout:\n{stdout}");
    assert!(stdout.contains("FAIL yt-dlp"), "stdout:\n{stdout}");
    assert!(stdout.contains("curl (optional)"), "stdout:\n{stdout}");
}

#[cfg(unix)]
#[test]
fn doctor_reports_an_unwritable_data_directory_without_probing_by_write() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("temp dir");
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o500))
        .expect("make read only");

    let output = doctor(temp.path());
    let stdout = String::from_utf8_lossy(&output.stdout);

    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700))
        .expect("restore permissions");
    assert!(
        stdout.contains("not a writable directory"),
        "stdout:\n{stdout}"
    );
}
