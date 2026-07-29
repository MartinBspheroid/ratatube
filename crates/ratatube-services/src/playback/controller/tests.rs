use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::mpsc;

use super::PlaybackController;
use crate::playback::{PlaybackEvent, ipc::MpvIpc};

#[tokio::test]
async fn repeated_volume_adjustments_accumulate_before_mpv_events_arrive() {
    let temp = tempfile::tempdir().expect("temp dir");
    let socket = temp.path().join("mpv.sock");
    let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept IPC client");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();
        while let Some(line) = lines.next_line().await.expect("read request") {
            let request: serde_json::Value = serde_json::from_str(&line).expect("request json");
            acknowledge(
                &mut writer,
                request["request_id"].as_u64().expect("request id"),
            )
            .await;
        }
    });
    let (event_tx, _event_rx) = mpsc::channel(8);
    let ipc = MpvIpc::connect(&socket, event_tx).await.expect("connect");
    let mut controller = PlaybackController::new(ipc);
    controller.on_event(&PlaybackEvent::VolumeChanged(50.0));

    controller.adjust_volume(2).await.expect("first increase");
    controller.adjust_volume(2).await.expect("second increase");

    assert_eq!(controller.snapshot().volume, 54);
    server.abort();
}

#[tokio::test]
async fn queued_volume_adjustments_remain_relative_after_a_stale_event() {
    let temp = tempfile::tempdir().expect("temp dir");
    let socket = temp.path().join("mpv.sock");
    let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept IPC client");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();
        let mut commands = Vec::new();
        while commands.len() < 2 {
            let line = lines
                .next_line()
                .await
                .expect("read request")
                .expect("request line");
            let request: serde_json::Value = serde_json::from_str(&line).expect("request json");
            commands.push(request["command"].clone());
            acknowledge(
                &mut writer,
                request["request_id"].as_u64().expect("request id"),
            )
            .await;
        }
        commands
    });
    let (event_tx, _event_rx) = mpsc::channel(8);
    let ipc = MpvIpc::connect(&socket, event_tx).await.expect("connect");
    let mut controller = PlaybackController::new(ipc);

    controller.queue_adjust_volume(2).expect("first increase");
    controller.on_event(&PlaybackEvent::VolumeChanged(0.0));
    controller.queue_adjust_volume(2).expect("second increase");

    assert_eq!(
        server.await.expect("server"),
        vec![
            serde_json::json!(["add", "volume", 2]),
            serde_json::json!(["add", "volume", 2]),
        ]
    );
}

#[tokio::test]
async fn queued_control_returns_before_delayed_acknowledgement() {
    let temp = tempfile::tempdir().expect("temp dir");
    let socket = temp.path().join("mpv.sock");
    let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept IPC client");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();
        let line = lines
            .next_line()
            .await
            .expect("read request")
            .expect("request line");
        let request: serde_json::Value = serde_json::from_str(&line).expect("request json");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        acknowledge(
            &mut writer,
            request["request_id"].as_u64().expect("request id"),
        )
        .await;
    });
    let (event_tx, _event_rx) = mpsc::channel(8);
    let ipc = MpvIpc::connect(&socket, event_tx).await.expect("connect");
    let controller = PlaybackController::new(ipc);
    let started = std::time::Instant::now();

    controller.queue_seek_to(12.0).expect("queue seek");

    assert!(started.elapsed() < std::time::Duration::from_millis(50));
    server.await.expect("server");
}

async fn acknowledge(writer: &mut tokio::net::unix::OwnedWriteHalf, request_id: u64) {
    writer
        .write_all(format!("{{\"request_id\":{request_id},\"error\":\"success\"}}\n").as_bytes())
        .await
        .expect("write acknowledgement");
}
