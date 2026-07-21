//! Daemon socket integration: handshake, commands, broadcast, shutdown.
//! Runs the real daemon loop in-process with mpv disabled (`"false"`) and a
//! missing yt-dlp, so no subprocesses or network are involved.

use tokio::io::BufReader;
use tokio::net::UnixStream;

use ytm_tui::config::Config;
use ytm_tui::media::Track;
use ytm_tui::persistence::AppPaths;
use ytm_tui::protocol::{
    self, ClientFrame, Command, DaemonFrame, ReplyBody, ReplyResult, WireEvent,
};

async fn connect_with_retry(socket: &std::path::Path) -> UnixStream {
    for _ in 0..200 {
        if let Ok(stream) = UnixStream::connect(socket).await {
            return stream;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("daemon socket never appeared at {}", socket.display());
}

#[tokio::test]
async fn daemon_serves_handshake_commands_events_and_shutdown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_data_dir(dir.path().to_path_buf());
    let mut config = Config::default();
    config.paths.mpv = "false".to_string();
    config.paths.yt_dlp = "/nonexistent/yt-dlp".to_string();
    let socket = ytm_tui::daemon::socket_path(&paths);
    let daemon = tokio::spawn(ytm_tui::daemon::run(paths, config, None));

    let stream = connect_with_retry(&socket).await;
    let (read_half, mut write) = stream.into_split();
    let mut read = BufReader::new(read_half);

    protocol::write_frame(
        &mut write,
        &ClientFrame::Hello {
            protocol: protocol::PROTOCOL_VERSION,
        },
    )
    .await
    .expect("send hello");
    let welcome: DaemonFrame = protocol::read_frame(&mut read)
        .await
        .expect("read welcome")
        .expect("welcome frame");
    match welcome {
        DaemonFrame::Welcome { protocol, snapshot } => {
            assert_eq!(protocol, protocol::PROTOCOL_VERSION);
            assert!(snapshot.queue.tracks.is_empty());
            assert!(!snapshot.health.yt_dlp_ready);
        }
        other => panic!("expected welcome, got {other:?}"),
    }

    let track = Track::new("id-1", "Daemon test track", "Channel");
    protocol::write_frame(
        &mut write,
        &ClientFrame::Command {
            id: 1,
            command: Box::new(Command::QueueAdd { track, next: false }),
        },
    )
    .await
    .expect("send queue add");

    let mut saw_reply = false;
    let mut saw_queue_event = false;
    while !(saw_reply && saw_queue_event) {
        let frame: DaemonFrame = protocol::read_frame(&mut read)
            .await
            .expect("read frame")
            .expect("frame before eof");
        match frame {
            DaemonFrame::Reply {
                id: 1,
                result: ReplyResult::Result(ReplyBody::Ack),
            } => saw_reply = true,
            DaemonFrame::Event { event } => {
                if let WireEvent::QueueChanged { queue, .. } = *event {
                    assert_eq!(queue.tracks.len(), 1);
                    saw_queue_event = true;
                }
            }
            other => panic!("unexpected frame {other:?}"),
        }
    }

    protocol::write_frame(
        &mut write,
        &ClientFrame::Command {
            id: 2,
            command: Box::new(Command::Status),
        },
    )
    .await
    .expect("send status");
    let status: DaemonFrame = protocol::read_frame(&mut read)
        .await
        .expect("read status")
        .expect("status frame");
    match status {
        DaemonFrame::Reply {
            id: 2,
            result: ReplyResult::Result(ReplyBody::Status { snapshot }),
        } => {
            assert_eq!(snapshot.queue.tracks.len(), 1);
            assert_eq!(snapshot.queue.tracks[0].title, "Daemon test track");
        }
        other => panic!("expected status reply, got {other:?}"),
    }

    protocol::write_frame(
        &mut write,
        &ClientFrame::Command {
            id: 3,
            command: Box::new(Command::Shutdown),
        },
    )
    .await
    .expect("send shutdown");
    let shutdown: DaemonFrame = protocol::read_frame(&mut read)
        .await
        .expect("read shutdown reply")
        .expect("shutdown reply frame");
    assert!(matches!(
        shutdown,
        DaemonFrame::Reply {
            id: 3,
            result: ReplyResult::Result(ReplyBody::Ack)
        }
    ));

    daemon
        .await
        .expect("daemon task join")
        .expect("daemon exits cleanly");
    assert!(!socket.exists(), "socket removed on shutdown");
}

#[tokio::test]
async fn second_daemon_refuses_while_first_owns_the_socket() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_data_dir(dir.path().to_path_buf());
    let mut config = Config::default();
    config.paths.mpv = "false".to_string();
    config.paths.yt_dlp = "/nonexistent/yt-dlp".to_string();
    let socket = ytm_tui::daemon::socket_path(&paths);
    let first = tokio::spawn(ytm_tui::daemon::run(paths.clone(), config.clone(), None));
    let stream = connect_with_retry(&socket).await;

    let second = ytm_tui::daemon::run(paths, config, None).await;
    assert!(second.is_err(), "second daemon must refuse");

    // Shut the first daemon down cleanly.
    let (read_half, mut write) = stream.into_split();
    let mut read = BufReader::new(read_half);
    protocol::write_frame(
        &mut write,
        &ClientFrame::Hello {
            protocol: protocol::PROTOCOL_VERSION,
        },
    )
    .await
    .expect("hello");
    let _welcome: DaemonFrame = protocol::read_frame(&mut read)
        .await
        .expect("read")
        .expect("welcome");
    protocol::write_frame(
        &mut write,
        &ClientFrame::Command {
            id: 1,
            command: Box::new(Command::Shutdown),
        },
    )
    .await
    .expect("shutdown");
    first.await.expect("join").expect("clean exit");
}
