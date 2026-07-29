use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use ratatube_services::media::yt_dlp::YtDlp;

/// Write a fake yt-dlp executable that emits canned output.
pub(super) fn mock_yt_dlp(script_body: &str) -> (tempfile::TempDir, YtDlp) {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("yt-dlp");
    let mut file = std::fs::File::create(&path).expect("create script");
    writeln!(file, "#!/bin/bash\n{script_body}").expect("write script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod script");
    let client = YtDlp::new(path.to_str().expect("utf8"));
    (directory, client)
}

/// Write a fake yt-dlp executable that records its positional arguments.
pub(super) fn capturing_yt_dlp() -> (tempfile::TempDir, YtDlp, PathBuf) {
    capturing_yt_dlp_with_output(r#"{"id":"video","title":"Title","entries":[]}"#)
}

/// Write a fake yt-dlp executable that records arguments and emits `output`.
pub(super) fn capturing_yt_dlp_with_output(output: &str) -> (tempfile::TempDir, YtDlp, PathBuf) {
    let directory = tempfile::tempdir().expect("tempdir");
    let binary = directory.path().join("yt-dlp");
    let arguments = directory.path().join("args.txt");
    let mut file = std::fs::File::create(&binary).expect("create script");
    writeln!(
        file,
        "#!/bin/bash\nprintf '%s\\n' \"$@\" > '{}'\nprintf '%s' '{}'",
        arguments.display(),
        output
    )
    .expect("write script");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("chmod script");
    let client = YtDlp::new(binary.to_str().expect("utf8"));
    (directory, client, arguments)
}

/// Read the positional arguments captured by [`capturing_yt_dlp`].
pub(super) fn captured_args(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("captured args")
        .lines()
        .map(str::to_string)
        .collect()
}
