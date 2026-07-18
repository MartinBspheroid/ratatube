//! yt-dlp integration tests: deterministic mocked binary plus live
//! network tests (ignored by default, run with `-- --ignored`).

use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use ytm_tui::media::yt_dlp::YtDlp;

/// Write a fake `yt-dlp` executable that emits canned JSON per argument set.
fn mock_yt_dlp(script_body: &str) -> (tempfile::TempDir, YtDlp) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("yt-dlp");
    let mut file = std::fs::File::create(&path).expect("create script");
    writeln!(file, "#!/bin/bash\n{script_body}").expect("write script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod script");
    let client = YtDlp::new(path.to_str().expect("utf8").to_string());
    (dir, client)
}

fn capturing_yt_dlp() -> (tempfile::TempDir, YtDlp, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let binary = dir.path().join("yt-dlp");
    let args = dir.path().join("args.txt");
    let mut file = std::fs::File::create(&binary).expect("create script");
    writeln!(
        file,
        "#!/bin/bash\nprintf '%s\\n' \"$@\" > '{}'\necho '{{\"id\":\"video\",\"title\":\"Title\",\"entries\":[]}}'",
        args.display()
    )
    .expect("write script");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("chmod script");
    let client = YtDlp::new(binary.to_str().expect("utf8"));
    (dir, client, args)
}

fn captured_args(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("captured args")
        .lines()
        .map(str::to_string)
        .collect()
}

const SEARCH_LINE: &str = r#"{"id":"vid1","title":"Song One","uploader":"Channel A","duration":213.0,"webpage_url":"https://www.youtube.com/watch?v=vid1"}"#;

#[tokio::test]
async fn search_parses_ndjson_lines() {
    let body = format!(
        "echo '{SEARCH_LINE}'\necho '{{\"id\":\"vid2\",\"title\":\"Song Two\",\"channel\":\"Channel B\",\"duration\":95}}'\necho 'not json at all'"
    );
    let (_dir, client) = mock_yt_dlp(&body);
    let tracks = client.search("anything", 5).await.expect("search");
    assert_eq!(
        tracks.len(),
        2,
        "unparseable line skipped (partial results)"
    );
    assert_eq!(tracks[0].id, "vid1");
    assert_eq!(tracks[0].artist, "Channel A");
    assert_eq!(tracks[0].duration_seconds, Some(213));
    assert_eq!(tracks[1].artist, "Channel B");
}

#[tokio::test]
async fn search_preserves_channel_identity() {
    let body = r#"echo '{"id":"video","title":"Title","channel":"Channel","channel_id":"UC123","channel_url":"https://www.youtube.com/channel/UC123"}'"#;
    let (_dir, client) = mock_yt_dlp(body);
    let tracks = client.search("anything", 1).await.expect("search");
    assert_eq!(tracks[0].channel_id.as_deref(), Some("UC123"));
    assert_eq!(
        tracks[0].channel_url.as_deref(),
        Some("https://www.youtube.com/channel/UC123")
    );
}

#[tokio::test]
async fn search_skips_deleted_and_private_entries() {
    let body = "echo '{\"id\":\"a\",\"title\":\"[Deleted video]\"}'\necho '{\"id\":\"b\",\"title\":\"Real Song\",\"uploader\":\"X\"}'";
    let (_dir, client) = mock_yt_dlp(body);
    let tracks = client.search("q", 5).await.expect("search");
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, "b");
}

#[tokio::test]
async fn fetch_playlist_flattens_entries() {
    let body = r#"cat <<'EOF'
{"title":"My Playlist","entries":[
  {"id":"e1","title":"Track 1","uploader":"A"},
  {"id":"e2","title":"Track 2","uploader":"B"}
]}
EOF"#;
    let (_dir, client) = mock_yt_dlp(body);
    let fetch = client.fetch_playlist("https://x").await.expect("playlist");
    assert_eq!(fetch.title, "My Playlist");
    assert_eq!(fetch.tracks.len(), 2);
}

#[tokio::test]
async fn playlist_rejections_preserve_each_normalization_reason() {
    let body = r#"cat <<'EOF'
{"title":"Mixed","entries":[
  {"id":"ok","title":"Track"},
  {"title":"No id"},
  {"id":"no-title"},
  {"id":"deleted","title":"[Deleted video]"},
  {"id":"private-title","title":"[Private video]"},
  {"id":"private-availability","title":"Hidden","availability":"private"},
  {"id":"unavailable","title":"Blocked","availability":"unavailable"}
]}
EOF"#;
    let (_dir, client) = mock_yt_dlp(body);

    let fetch = client.fetch_playlist("https://x").await.expect("playlist");

    assert_eq!(fetch.tracks.len(), 1);
    assert_eq!(fetch.rejections.missing_id, 1);
    assert_eq!(fetch.rejections.missing_title, 1);
    assert_eq!(fetch.rejections.deleted, 1);
    assert_eq!(fetch.rejections.private, 2);
    assert_eq!(fetch.rejections.unavailable, 1);
}

#[tokio::test]
async fn resolve_stream_returns_first_line() {
    let (_dir, client) =
        mock_yt_dlp("echo 'https://googlevideo.com/stream?sig=SECRET'\necho 'extra'");
    let url = client
        .resolve_stream("https://www.youtube.com/watch?v=x")
        .await
        .expect("resolve");
    assert_eq!(url, "https://googlevideo.com/stream?sig=SECRET");
}

#[tokio::test]
async fn invalid_json_from_yt_dlp_is_an_error() {
    let (_dir, client) = mock_yt_dlp("echo 'garbage'");
    let result = client.fetch_playlist("https://x").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn nonzero_exit_maps_to_ytdlp_error() {
    let (_dir, client) = mock_yt_dlp("echo 'ERROR: Video unavailable' >&2\nexit 1");
    let err = client.search("q", 1).await.expect_err("should fail");
    assert!(err.to_string().contains("Video unavailable"));
}

#[tokio::test]
async fn missing_binary_is_structured_error() {
    let client = YtDlp::new("/nonexistent/yt-dlp".to_string());
    let err = client.search("q", 1).await.expect_err("missing binary");
    assert!(matches!(
        err,
        ytm_tui::error::AppError::MissingDependency(bin) if bin == "/nonexistent/yt-dlp"
    ));
}

#[tokio::test]
async fn every_positional_input_is_separated_from_yt_dlp_options() {
    let (_dir, client, args) = capturing_yt_dlp();
    client.search("--weird query", 3).await.expect("search");
    let search_args = captured_args(&args);
    assert_eq!(
        &search_args[search_args.len() - 2..],
        &["--", "ytsearch3:--weird query"]
    );

    let (_dir, client, args) = capturing_yt_dlp();
    client
        .fetch_video("https://www.youtube.com/watch?v=x")
        .await
        .expect("video");
    let video_args = captured_args(&args);
    assert_eq!(
        &video_args[video_args.len() - 2..],
        &["--", "https://www.youtube.com/watch?v=x"]
    );

    let (_dir, client, args) = capturing_yt_dlp();
    client
        .fetch_details("https://www.youtube.com/watch?v=x")
        .await
        .expect("details");
    let details_args = captured_args(&args);
    assert_eq!(
        &details_args[details_args.len() - 2..],
        &["--", "https://www.youtube.com/watch?v=x"]
    );

    let (_dir, client, args) = capturing_yt_dlp();
    client
        .fetch_playlist("https://www.youtube.com/playlist?list=PLx")
        .await
        .expect("playlist");
    let playlist_args = captured_args(&args);
    assert_eq!(
        &playlist_args[playlist_args.len() - 2..],
        &["--", "https://www.youtube.com/playlist?list=PLx"]
    );

    let (_dir, client, args) = capturing_yt_dlp();
    client
        .resolve_stream("https://www.youtube.com/watch?v=x")
        .await
        .expect("stream");
    let stream_args = captured_args(&args);
    assert_eq!(
        &stream_args[stream_args.len() - 2..],
        &["--", "https://www.youtube.com/watch?v=x"]
    );
}

#[tokio::test]
async fn subprocess_stdout_limit_accepts_exact_size_and_rejects_one_byte_over() {
    let (_dir, exact) = mock_yt_dlp("printf '1234'");
    exact
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect("exact stdout limit");

    let (_dir, oversized) = mock_yt_dlp("printf '12345'");
    let error = oversized
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect_err("stdout over limit");
    assert!(matches!(
        error,
        ytm_tui::error::AppError::ResourceLimit { ref resource, limit: 4 }
            if resource == "yt-dlp stdout"
    ));
}

#[tokio::test]
async fn subprocess_stderr_limit_accepts_exact_size_and_rejects_one_byte_over() {
    let (_dir, exact) = mock_yt_dlp("printf '1234' >&2\nexit 1");
    let exact_error = exact
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect_err("nonzero exit");
    assert!(matches!(exact_error, ytm_tui::error::AppError::YtDlp(_)));

    let (_dir, oversized) = mock_yt_dlp("printf '12345' >&2\nexit 1");
    let error = oversized
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect_err("stderr over limit");
    assert!(matches!(
        error,
        ytm_tui::error::AppError::ResourceLimit { ref resource, limit: 4 }
            if resource == "yt-dlp stderr"
    ));
}

// --- Live network tests (PRD manual test matrix) ---------------------------

#[tokio::test]
#[ignore = "requires network and real yt-dlp"]
async fn live_search_youtube() {
    if which::which("yt-dlp").is_err() {
        return;
    }
    let client = YtDlp::new("yt-dlp".to_string());
    let tracks = client
        .search("massive attack teardrop", 3)
        .await
        .expect("search");
    assert!(!tracks.is_empty(), "live search returned results");
    assert!(tracks.iter().all(|t| !t.id.is_empty()));
}

#[tokio::test]
#[ignore = "requires network and real yt-dlp"]
async fn live_resolve_stream() {
    if which::which("yt-dlp").is_err() {
        return;
    }
    let client = YtDlp::new("yt-dlp".to_string());
    let url = client
        .resolve_stream("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        .await
        .expect("resolve");
    assert!(url.starts_with("http"), "got a playable URL");
}

#[tokio::test]
#[ignore = "requires network and real yt-dlp"]
async fn live_fetch_playlist() {
    if which::which("yt-dlp").is_err() {
        return;
    }
    let client = YtDlp::new("yt-dlp".to_string());
    let fetch = client
        .fetch_playlist("https://www.youtube.com/playlist?list=PLFgquLnL59alCl_2TQvOiD5Vgm1hCaGSI")
        .await
        .expect("playlist");
    assert!(!fetch.tracks.is_empty());
}
