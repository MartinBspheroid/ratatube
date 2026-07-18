//! End-to-end mpv IPC test: spawn mpv, load a real audio file, and verify
//! playback events and controls. Skipped when mpv or ffmpeg is unavailable.

use std::path::PathBuf;
use std::time::Duration;

use ytm_tui::playback::controller::PlaybackController;
use ytm_tui::playback::ipc::MpvIpc;
use ytm_tui::playback::{MpvProcess, PlaybackEvent};

fn binary_available(name: &str) -> bool {
    which::which(name).is_ok()
}

/// Generate a 5-second sine wave audio file for playback.
fn generate_audio_fixture(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("tone.wav");
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=5",
            path.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("run ffmpeg");
    assert!(status.status.success(), "ffmpeg fixture generation failed");
    path
}

#[tokio::test]
async fn mpv_plays_audio_and_reports_events() {
    if !binary_available("mpv") || !binary_available("ffmpeg") {
        eprintln!("skipping: mpv/ffmpeg not installed");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let audio = generate_audio_fixture(dir.path());
    let socket = dir.path().join("mpv.sock");

    let mut process = MpvProcess::spawn("mpv", &socket, 70).expect("spawn mpv");
    process
        .wait_for_socket(Duration::from_secs(5))
        .await
        .expect("socket appears");

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<PlaybackEvent>(64);
    let ipc = MpvIpc::connect(&socket, event_tx)
        .await
        .expect("connect ipc");
    let mut controller = PlaybackController::new(ipc);
    controller.observe_defaults().await.expect("observe");

    controller
        .load(audio.to_str().expect("utf8"), "test tone")
        .await
        .expect("loadfile");

    // Collect events until playback starts and position advances.
    let mut started = false;
    let mut saw_position = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline && !(started && saw_position) {
        let event = tokio::time::timeout(Duration::from_secs(3), event_rx.recv())
            .await
            .expect("event within timeout")
            .expect("channel open");
        match event {
            PlaybackEvent::Started => started = true,
            PlaybackEvent::PositionChanged(p) if p > 0.0 => saw_position = true,
            _ => {}
        }
    }
    assert!(started, "mpv reported playback start");
    assert!(saw_position, "mpv reported advancing position");

    // Controls: pause, seek, volume, mute.
    controller.pause().await.expect("pause");
    controller.seek_to(1.0).await.expect("seek");
    controller.set_volume(0).await.expect("reset volume");
    controller
        .queue_adjust_volume(2)
        .expect("first queued volume increase");
    controller
        .queue_adjust_volume(2)
        .expect("second queued volume increase");
    let saw_four_percent = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(event) = event_rx.recv().await {
            if matches!(event, PlaybackEvent::VolumeChanged(value) if (value - 4.0).abs() < 0.1) {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    assert!(saw_four_percent, "two queued + presses reached 4% in mpv");
    controller.toggle_mute().await.expect("mute");

    // Graceful quit.
    controller.quit().await.expect("quit");
    process.shutdown().await;
    assert!(!socket.exists(), "socket cleaned up");
}
