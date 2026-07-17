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
