use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use super::MpvIpc;
use super::frame::parse_frame;
use crate::playback::events::PlaybackEvent;
use ratatube_domain::error::AppError;

async fn test_client(
    timeout: Duration,
) -> (
    MpvIpc,
    mpsc::Receiver<PlaybackEvent>,
    tempfile::TempDir,
    UnixListener,
) {
    let temp = tempfile::tempdir().expect("temp dir");
    let socket = temp.path().join("mpv.sock");
    let listener = UnixListener::bind(&socket).expect("bind fake mpv socket");
    let (event_tx, event_rx) = mpsc::channel(8);
    let client = MpvIpc::connect_with_timeout(&socket, event_tx, timeout)
        .await
        .expect("connect fake mpv");
    (client, event_rx, temp, listener)
}

async fn read_request(lines: &mut tokio::io::Lines<BufReader<UnixStream>>) -> Value {
    let line = lines
        .next_line()
        .await
        .expect("read request")
        .expect("request line");
    serde_json::from_str(&line).expect("request json")
}

#[test]
fn parses_end_file() {
    assert_eq!(
        parse_frame(r#"{"event":"end-file","reason":"eof"}"#),
        Some(PlaybackEvent::EndFile {
            reason: "eof".to_string(),
        })
    );
}

#[test]
fn parses_position_change() {
    assert_eq!(
        parse_frame(r#"{"event":"property-change","name":"time-pos","data":12.5}"#),
        Some(PlaybackEvent::PositionChanged(12.5))
    );
}

#[test]
fn parses_astats_metadata_into_audio_levels() {
    let line = r#"{"event":"property-change","id":7,"name":"af-metadata/vis","data":{
        "lavfi.astats.1.RMS_level":"-21.0","lavfi.astats.1.Peak_level":"-18.0",
        "lavfi.astats.1.Zero_crossings_rate":"0.020",
        "lavfi.astats.2.RMS_level":"-23.0","lavfi.astats.2.Peak_level":"-16.0",
        "lavfi.astats.2.Zero_crossings_rate":"0.040",
        "lavfi.astats.Overall.RMS_level":"-22.0"}}"#;
    let Some(PlaybackEvent::AudioLevels(levels)) = parse_frame(line) else {
        panic!("expected audio levels");
    };
    assert_eq!(levels.rms_db, -22.0);
    assert_eq!(levels.peak_db, -16.0);
    assert!((levels.zcr - 0.030).abs() < 1e-6);
}

#[test]
fn silence_metadata_clamps_negative_infinity_to_the_floor() {
    let line = r#"{"event":"property-change","name":"af-metadata/vis","data":{
        "lavfi.astats.1.RMS_level":"-inf","lavfi.astats.1.Peak_level":"-inf",
        "lavfi.astats.1.Zero_crossings_rate":"0.000",
        "lavfi.astats.Overall.RMS_level":"-inf"}}"#;
    let Some(PlaybackEvent::AudioLevels(levels)) = parse_frame(line) else {
        panic!("expected audio levels");
    };
    assert_eq!(levels.rms_db, -90.0);
    assert_eq!(levels.peak_db, -90.0);
}

#[test]
fn parses_file_loaded_as_media_boundary() {
    assert_eq!(
        parse_frame(r#"{"event":"file-loaded"}"#),
        Some(PlaybackEvent::FileLoaded)
    );
}

#[test]
fn preserves_playback_restart_as_started_status() {
    assert_eq!(
        parse_frame(r#"{"event":"playback-restart"}"#),
        Some(PlaybackEvent::Started)
    );
}

#[test]
fn parses_speed_change() {
    assert_eq!(
        parse_frame(r#"{"event":"property-change","name":"speed","data":1.5}"#),
        Some(PlaybackEvent::SpeedChanged(1.5))
    );
}

#[test]
fn ignores_unknown_frames() {
    assert!(parse_frame(r#"{"event":"video-reconfig"}"#).is_none());
    assert!(parse_frame("not json").is_none());
}

#[tokio::test]
async fn command_returns_matching_success_data() {
    let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut lines = BufReader::new(stream).lines();
        let request_id = read_request(&mut lines).await["request_id"]
            .as_u64()
            .expect("request id");
        lines
            .into_inner()
            .into_inner()
            .write_all(
                format!("{{\"request_id\":{request_id},\"error\":\"success\",\"data\":7}}\n")
                    .as_bytes(),
            )
            .await
            .expect("respond");
    });
    let response = client
        .command(vec![json!("get_property"), json!("volume")])
        .await
        .expect("acknowledged command");
    assert_eq!(response, json!(7));
    server.await.expect("server");
}

#[tokio::test]
async fn command_surfaces_matching_mpv_error() {
    let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut lines = BufReader::new(stream).lines();
        let request_id = read_request(&mut lines).await["request_id"]
            .as_u64()
            .expect("request id");
        lines
            .into_inner()
            .into_inner()
            .write_all(
                format!("{{\"request_id\":{request_id},\"error\":\"invalid parameter\"}}\n")
                    .as_bytes(),
            )
            .await
            .expect("respond");
    });
    let error = client.command(vec![json!("bad")]).await.expect_err("error");
    assert!(matches!(error, AppError::MpvPlayback(message) if message == "invalid parameter"));
}

#[tokio::test]
async fn reordered_responses_reach_their_own_request() {
    let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut lines = BufReader::new(stream).lines();
        let first_id = read_request(&mut lines).await["request_id"]
            .as_u64()
            .unwrap();
        let second_id = read_request(&mut lines).await["request_id"]
            .as_u64()
            .unwrap();
        lines.into_inner().into_inner().write_all(format!(
            "{{\"request_id\":{second_id},\"error\":\"success\",\"data\":\"second\"}}\n{{\"request_id\":{first_id},\"error\":\"success\",\"data\":\"first\"}}\n"
        ).as_bytes()).await.expect("respond");
    });
    let first_client = client.clone();
    let second_client = client.clone();
    let (first, second) = tokio::join!(
        first_client.command(vec![json!("first")]),
        second_client.command(vec![json!("second")])
    );
    assert_eq!(first.expect("first"), json!("first"));
    assert_eq!(second.expect("second"), json!("second"));
}

#[tokio::test]
async fn command_times_out_without_a_response() {
    let (client, _events, _temp, listener) = test_client(Duration::from_millis(20)).await;
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.expect("accept");
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let error = client
        .command(vec![json!("silent")])
        .await
        .expect_err("timeout");
    assert!(matches!(error, AppError::Timeout(_)));
    server.abort();
}

#[tokio::test]
async fn malformed_frame_fails_pending_command() {
    let (client, _events, _temp, listener) = test_client(Duration::from_secs(1)).await;
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        stream.write_all(b"{bad json}\n").await.expect("write");
    });
    let error = client
        .command(vec![json!("malformed")])
        .await
        .expect_err("error");
    assert!(matches!(error, AppError::MpvIpc(_)));
}

#[tokio::test]
async fn eof_fails_pending_command_and_emits_shutdown() {
    let (client, mut events, _temp, listener) = test_client(Duration::from_secs(1)).await;
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        drop(stream);
    });
    let error = client
        .command(vec![json!("disconnect")])
        .await
        .expect_err("error");
    assert!(matches!(error, AppError::MpvIpc(_)));
    assert_eq!(events.recv().await, Some(PlaybackEvent::Shutdown));
}
