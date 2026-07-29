use ratatube_services::media::yt_dlp::YtDlp;

#[tokio::test]
#[ignore = "requires network and real yt-dlp"]
async fn live_search_youtube() {
    if which::which("yt-dlp").is_err() {
        return;
    }
    let client = YtDlp::new("yt-dlp");
    let tracks = client
        .search("massive attack teardrop", 3)
        .await
        .expect("search");
    assert!(!tracks.is_empty(), "live search returned results");
    assert!(tracks.iter().all(|track| !track.id.is_empty()));
}

#[tokio::test]
#[ignore = "requires network and real yt-dlp"]
async fn live_resolve_stream() {
    if which::which("yt-dlp").is_err() {
        return;
    }
    let client = YtDlp::new("yt-dlp");
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
    let client = YtDlp::new("yt-dlp");
    let fetch = client
        .fetch_playlist("https://www.youtube.com/playlist?list=PLFgquLnL59alCl_2TQvOiD5Vgm1hCaGSI")
        .await
        .expect("playlist");
    assert!(!fetch.tracks.is_empty());
}
