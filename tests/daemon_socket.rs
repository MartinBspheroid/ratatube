//! Daemon socket integration: handshake, commands, broadcast, shutdown.
//! Runs the real daemon loop in-process with mpv disabled (`"false"`) and a
//! missing yt-dlp, so no subprocesses or network are involved.

use tokio::io::BufReader;
use tokio::net::UnixStream;

use ratatube::config::Config;
use ratatube::media::Track;
use ratatube::persistence::AppPaths;
use ratatube::protocol::{
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
    let socket = ratatube::daemon::socket_path(&paths);
    let daemon = tokio::spawn(ratatube::daemon::run(paths, config, None));

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
async fn client_connection_requests_against_a_live_daemon() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_data_dir(dir.path().to_path_buf());
    let mut config = Config::default();
    config.paths.mpv = "false".to_string();
    config.paths.yt_dlp = "/nonexistent/yt-dlp".to_string();
    let socket = ratatube::daemon::socket_path(&paths);
    let daemon = tokio::spawn(ratatube::daemon::run(paths, config, None));

    let mut connection = None;
    for _ in 0..200 {
        match ratatube::client::Connection::connect(&socket).await {
            Ok(conn) => {
                connection = Some(conn);
                break;
            }
            Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
        }
    }
    let mut connection = connection.expect("client connects");
    assert!(connection.snapshot.queue.tracks.is_empty());

    let body = connection
        .request(Command::QueueAdd {
            track: Track::new("id-9", "Client track", "Channel"),
            next: false,
        })
        .await
        .expect("queue add reply");
    assert!(matches!(body, ReplyBody::Ack));

    let body = connection
        .request(Command::PlaylistCreate {
            name: "Wire playlist".into(),
        })
        .await
        .expect("playlist create reply");
    assert!(matches!(body, ReplyBody::Ack));

    // Broadcasts (queue_changed, playlists_changed) sit between the
    // replies; request() must skip them and still correlate.
    let ReplyBody::Status { snapshot } = connection
        .request(Command::Status)
        .await
        .expect("status reply")
    else {
        panic!("expected status body");
    };
    assert_eq!(snapshot.queue.tracks.len(), 1);
    assert_eq!(snapshot.playlists.len(), 1);
    assert_eq!(snapshot.playlists[0].name, "Wire playlist");

    connection
        .request(Command::Shutdown)
        .await
        .expect("shutdown reply");
    daemon.await.expect("join").expect("clean exit");
}

#[tokio::test]
async fn broadcasts_reach_every_connected_client() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_data_dir(dir.path().to_path_buf());
    let mut config = Config::default();
    config.paths.mpv = "false".to_string();
    config.paths.yt_dlp = "/nonexistent/yt-dlp".to_string();
    let socket = ratatube::daemon::socket_path(&paths);
    let daemon = tokio::spawn(ratatube::daemon::run(paths, config, None));

    // Client A drives commands; client B only observes broadcasts.
    let stream_b = connect_with_retry(&socket).await;
    let (read_b, mut write_b) = stream_b.into_split();
    let mut read_b = BufReader::new(read_b);
    protocol::write_frame(
        &mut write_b,
        &ClientFrame::Hello {
            protocol: protocol::PROTOCOL_VERSION,
        },
    )
    .await
    .expect("b hello");
    let _welcome_b: DaemonFrame = protocol::read_frame(&mut read_b)
        .await
        .expect("b read")
        .expect("b welcome");

    let mut a = ratatube::client::Connection::connect(&socket)
        .await
        .expect("a connects");
    a.request(Command::QueueAdd {
        track: Track::new("id-b", "Broadcast track", "Channel"),
        next: false,
    })
    .await
    .expect("a queue add");

    let mut b_saw_queue_change = false;
    for _ in 0..8 {
        let frame: DaemonFrame = protocol::read_frame(&mut read_b)
            .await
            .expect("b read frame")
            .expect("b frame");
        if let DaemonFrame::Event { event } = frame
            && let WireEvent::QueueChanged { queue, .. } = *event
        {
            assert_eq!(queue.tracks.len(), 1);
            b_saw_queue_change = true;
            break;
        }
    }
    assert!(b_saw_queue_change, "observer client missed the broadcast");

    a.request(Command::Shutdown).await.expect("shutdown");
    daemon.await.expect("join").expect("clean exit");
}

#[tokio::test]
async fn slow_clients_are_disconnected_instead_of_blocking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_data_dir(dir.path().to_path_buf());
    let mut config = Config::default();
    config.paths.mpv = "false".to_string();
    config.paths.yt_dlp = "/nonexistent/yt-dlp".to_string();
    let socket = ratatube::daemon::socket_path(&paths);
    let daemon = tokio::spawn(ratatube::daemon::run(paths, config, None));

    // The slow client completes its handshake and then never reads.
    let stream_slow = connect_with_retry(&socket).await;
    let (read_slow, mut write_slow) = stream_slow.into_split();
    let mut read_slow = BufReader::new(read_slow);
    protocol::write_frame(
        &mut write_slow,
        &ClientFrame::Hello {
            protocol: protocol::PROTOCOL_VERSION,
        },
    )
    .await
    .expect("slow hello");
    let _welcome: DaemonFrame = protocol::read_frame(&mut read_slow)
        .await
        .expect("slow read")
        .expect("slow welcome");

    let mut fast = ratatube::client::Connection::connect(&socket)
        .await
        .expect("fast connects");
    for id in ["one", "two"] {
        fast.request(Command::QueueAdd {
            track: Track::new(id, id, "Channel"),
            next: false,
        })
        .await
        .expect("seed queue");
    }
    // Small queue_changed broadcasts overflow the slow client's bounded
    // outbound queue (1024 frames) without blocking the daemon; the round
    // count must exceed that bound for the drop to trigger.
    for round in 0..1300 {
        let (from, to) = if round % 2 == 0 { (0, 1) } else { (1, 0) };
        fast.request(Command::QueueMove { from, to })
            .await
            .expect("fast keeps working while the slow client lags");
    }

    // The daemon dropped the slow client: after the buffered frames drain,
    // its stream ends even though it never misbehaved on the wire.
    let mut ended = false;
    for _ in 0..3000 {
        match protocol::read_frame::<_, DaemonFrame>(&mut read_slow).await {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                ended = true;
                break;
            }
        }
    }
    assert!(ended, "slow client stream should end after disconnect");

    fast.request(Command::Shutdown).await.expect("shutdown");
    daemon.await.expect("join").expect("clean exit");
}

#[tokio::test]
async fn second_daemon_refuses_while_first_owns_the_socket() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_data_dir(dir.path().to_path_buf());
    let mut config = Config::default();
    config.paths.mpv = "false".to_string();
    config.paths.yt_dlp = "/nonexistent/yt-dlp".to_string();
    let socket = ratatube::daemon::socket_path(&paths);
    let first = tokio::spawn(ratatube::daemon::run(paths.clone(), config.clone(), None));
    let stream = connect_with_retry(&socket).await;

    let second = ratatube::daemon::run(paths, config, None).await;
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
