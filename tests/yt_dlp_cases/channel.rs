use ytm_tui::media::channel::{CHANNEL_PAGE_SIZE, ChannelPageRequest};

use super::support::{captured_args, capturing_yt_dlp_with_output, mock_yt_dlp};

#[tokio::test]
async fn topic_channel_without_videos_tab_falls_back_to_channel_root() {
    let script = r#"
last="${!#}"
if [[ "$last" == */videos ]]; then
  printf '%s\n' 'ERROR: This channel does not have a videos tab' >&2
  exit 1
fi
printf '%s\n' '{"id":"topic-video","title":"Topic upload","channel":"Artist - Topic"}'
"#;
    let (_directory, client) = mock_yt_dlp(script);
    let page = client
        .fetch_channel_page(&ChannelPageRequest {
            channel_url: "https://www.youtube.com/channel/UCTopic".into(),
            page: 0,
        })
        .await
        .expect("topic channel root fallback");

    assert_eq!(page.tracks[0].id, "topic-video");
}

#[tokio::test]
async fn channel_page_uses_exact_zero_based_bounds_and_normalized_url() {
    for (page, start, end) in [(0, "1", "30"), (1, "31", "60")] {
        let (_directory, client, arguments) = capturing_yt_dlp_with_output("");
        let request = ChannelPageRequest {
            channel_url: "https://youtube.com/@artist/videos/".into(),
            page,
        };
        client
            .fetch_channel_page(&request)
            .await
            .expect("channel page");
        assert_eq!(
            captured_args(&arguments),
            [
                "--dump-json",
                "--flat-playlist",
                "--no-download",
                "--ignore-errors",
                "--playlist-start",
                start,
                "--playlist-end",
                end,
                "--",
                "https://www.youtube.com/@artist/videos",
            ]
        );
    }
}

#[tokio::test]
async fn channel_page_preserves_partial_results_and_structured_rejections() {
    let output = concat!(
        "{\"id\":\"new\",\"title\":\"Newest\",\"channel\":\"Artist\"}\n",
        "malformed\n",
        "{\"id\":\"private\",\"title\":\"[Private video]\"}\n",
        "{\"id\":\"new\",\"title\":\"Boundary overlap\"}\n",
        "{\"id\":\"old\",\"title\":\"Older\"}\n",
    );
    let (_directory, client) = mock_yt_dlp(&format!("printf '%s' '{}'", output.trim_end()));
    let page = client
        .fetch_channel_page(&ChannelPageRequest {
            channel_url: "https://www.youtube.com/channel/UC1".into(),
            page: 0,
        })
        .await
        .expect("partial page");
    assert_eq!(
        page.tracks
            .iter()
            .map(|track| track.id.as_str())
            .collect::<Vec<_>>(),
        ["new", "old"]
    );
    assert_eq!(page.rejections.malformed, 1);
    assert_eq!(page.rejections.private, 1);
    assert!(page.exhausted);
}

#[tokio::test]
async fn rejected_rows_do_not_false_signal_exhaustion() {
    let output = (0..CHANNEL_PAGE_SIZE)
        .map(|index| format!("{{\"id\":\"{index}\",\"title\":\"[Private video]\"}}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (_directory, client) = mock_yt_dlp(&format!("printf '%s' '{}'", output));
    let page = client
        .fetch_channel_page(&ChannelPageRequest {
            channel_url: "https://youtube.com/@artist".into(),
            page: 0,
        })
        .await
        .expect("full source page");
    assert!(page.tracks.is_empty());
    assert_eq!(page.rejections.private, CHANNEL_PAGE_SIZE);
    assert!(!page.exhausted);
}

#[tokio::test]
async fn unsafe_channel_url_is_rejected_before_process_spawn() {
    let client = ytm_tui::media::yt_dlp::YtDlp::new("/nonexistent/yt-dlp");
    let error = client
        .fetch_channel_page(&ChannelPageRequest {
            channel_url: "https://youtube.com.evil.example/channel/UC1".into(),
            page: 0,
        })
        .await
        .expect_err("unsafe URL");
    assert!(matches!(error, ytm_tui::error::AppError::InvalidUrl(_)));
}
