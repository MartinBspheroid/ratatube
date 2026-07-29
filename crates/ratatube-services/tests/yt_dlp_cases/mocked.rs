use ratatube_services::media::yt_dlp::YtDlp;

use super::support::{captured_args, capturing_yt_dlp, mock_yt_dlp};

const SEARCH_LINE: &str = r#"{"id":"vid1","title":"Song One","uploader":"Channel A","duration":213.0,"webpage_url":"https://www.youtube.com/watch?v=vid1"}"#;

#[tokio::test]
async fn search_parses_ndjson_lines() {
    let body = format!(
        "echo '{SEARCH_LINE}'\necho '{{\"id\":\"vid2\",\"title\":\"Song Two\",\"channel\":\"Channel B\",\"duration\":95}}'\necho 'not json at all'"
    );
    let (_directory, client) = mock_yt_dlp(&body);
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
    let (_directory, client) = mock_yt_dlp(body);
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
    let (_directory, client) = mock_yt_dlp(body);
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
    let (_directory, client) = mock_yt_dlp(body);
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
    let (_directory, client) = mock_yt_dlp(body);
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
    let (_directory, client) =
        mock_yt_dlp("echo 'https://googlevideo.com/stream?sig=SECRET'\necho 'extra'");
    let url = client
        .resolve_stream("https://www.youtube.com/watch?v=x")
        .await
        .expect("resolve");
    assert_eq!(url, "https://googlevideo.com/stream?sig=SECRET");
}

#[tokio::test]
async fn invalid_json_from_yt_dlp_is_an_error() {
    let (_directory, client) = mock_yt_dlp("echo 'garbage'");
    assert!(client.fetch_playlist("https://x").await.is_err());
}

#[tokio::test]
async fn nonzero_exit_maps_to_ytdlp_error() {
    let (_directory, client) = mock_yt_dlp("echo 'ERROR: Video unavailable' >&2\nexit 1");
    let error = client.search("q", 1).await.expect_err("should fail");
    assert!(error.to_string().contains("Video unavailable"));
}

#[tokio::test]
async fn missing_binary_is_structured_error() {
    let client = YtDlp::new("/nonexistent/yt-dlp");
    let error = client.search("q", 1).await.expect_err("missing binary");
    assert!(matches!(
        error,
        ratatube_services::error::AppError::MissingDependency(binary) if binary == "/nonexistent/yt-dlp"
    ));
}

#[tokio::test]
async fn every_positional_input_is_separated_from_yt_dlp_options() {
    let (_directory, client, arguments) = capturing_yt_dlp();
    client.search("--weird query", 3).await.expect("search");
    assert_eq!(
        &captured_args(&arguments)[captured_args(&arguments).len() - 2..],
        &["--", "ytsearch3:--weird query"]
    );

    let (_directory, client, arguments) = capturing_yt_dlp();
    client
        .fetch_video("https://www.youtube.com/watch?v=x")
        .await
        .expect("video");
    assert_eq!(
        &captured_args(&arguments)[captured_args(&arguments).len() - 2..],
        &["--", "https://www.youtube.com/watch?v=x"]
    );

    let (_directory, client, arguments) = capturing_yt_dlp();
    client
        .fetch_details("https://www.youtube.com/watch?v=x")
        .await
        .expect("details");
    assert_eq!(
        &captured_args(&arguments)[captured_args(&arguments).len() - 2..],
        &["--", "https://www.youtube.com/watch?v=x"]
    );

    let (_directory, client, arguments) = capturing_yt_dlp();
    client
        .fetch_playlist("https://www.youtube.com/playlist?list=PLx")
        .await
        .expect("playlist");
    assert_eq!(
        &captured_args(&arguments)[captured_args(&arguments).len() - 2..],
        &["--", "https://www.youtube.com/playlist?list=PLx"]
    );

    let (_directory, client, arguments) = capturing_yt_dlp();
    client
        .resolve_stream("https://www.youtube.com/watch?v=x")
        .await
        .expect("stream");
    assert_eq!(
        &captured_args(&arguments)[captured_args(&arguments).len() - 2..],
        &["--", "https://www.youtube.com/watch?v=x"]
    );
}

#[tokio::test]
async fn subprocess_stdout_limit_accepts_exact_size_and_rejects_one_byte_over() {
    let (_directory, exact) = mock_yt_dlp("printf '1234'");
    exact
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect("exact stdout limit");

    let (_directory, oversized) = mock_yt_dlp("printf '12345'");
    let error = oversized
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect_err("stdout over limit");
    assert!(matches!(
        error,
        ratatube_services::error::AppError::ResourceLimit { ref resource, limit: 4 }
            if resource == "yt-dlp stdout"
    ));
}

#[tokio::test]
async fn subprocess_stderr_limit_accepts_exact_size_and_rejects_one_byte_over() {
    let (_directory, exact) = mock_yt_dlp("printf '1234' >&2\nexit 1");
    let exact_error = exact
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect_err("nonzero exit");
    assert!(matches!(
        exact_error,
        ratatube_services::error::AppError::YtDlp(_)
    ));

    let (_directory, oversized) = mock_yt_dlp("printf '12345' >&2\nexit 1");
    let error = oversized
        .with_output_limits(4, 4)
        .search("q", 1)
        .await
        .expect_err("stderr over limit");
    assert!(matches!(
        error,
        ratatube_services::error::AppError::ResourceLimit { ref resource, limit: 4 }
            if resource == "yt-dlp stderr"
    ));
}
