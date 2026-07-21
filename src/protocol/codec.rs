//! NDJSON framing: one JSON object per line with a hard size bound.

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum serialized frame size, matching the persistence document bound.
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Serialize one frame and write it as a single newline-terminated line.
pub async fn write_frame<W, T>(writer: &mut W, frame: &T) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let mut bytes = serde_json::to_vec(frame).map_err(invalid_data)?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(oversized(bytes.len()));
    }
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}

/// Read one line and parse it as `T`. Returns `None` on clean EOF at a
/// frame boundary; oversized or malformed frames are errors.
pub async fn read_frame<R, T>(reader: &mut R) -> std::io::Result<Option<T>>
where
    R: AsyncBufRead + Unpin,
    T: DeserializeOwned,
{
    let mut line = Vec::new();
    let mut limited = reader.take((MAX_FRAME_BYTES + 1) as u64);
    let read = limited.read_until(b'\n', &mut line).await?;
    if read == 0 {
        return Ok(None);
    }
    let content = if line.last() == Some(&b'\n') {
        &line[..line.len() - 1]
    } else {
        &line[..]
    };
    if content.len() > MAX_FRAME_BYTES {
        return Err(oversized(content.len()));
    }
    serde_json::from_slice(content)
        .map(Some)
        .map_err(invalid_data)
}

fn invalid_data(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
}

fn oversized(len: usize) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("frame of {len} bytes exceeds the {MAX_FRAME_BYTES}-byte bound"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ClientFrame, Command, DaemonFrame, ReplyBody, ReplyResult, WireEvent};

    async fn round_trip<T>(frame: &T) -> T
    where
        T: Serialize + DeserializeOwned,
    {
        let mut buffer = Vec::new();
        write_frame(&mut buffer, frame).await.expect("write");
        let mut reader = std::io::Cursor::new(buffer);
        read_frame(&mut reader).await.expect("read").expect("frame")
    }

    #[tokio::test]
    async fn client_frames_round_trip() {
        let hello: ClientFrame = round_trip(&ClientFrame::Hello { protocol: 1 }).await;
        assert!(matches!(hello, ClientFrame::Hello { protocol: 1 }));
        let command = round_trip(&ClientFrame::Command {
            id: 7,
            command: Box::new(Command::QueueAdd {
                track: crate::media::Track::new("id", "title", "channel"),
                next: true,
            }),
        })
        .await;
        match command {
            ClientFrame::Command { id: 7, command } => match *command {
                Command::QueueAdd { track, next: true } => assert_eq!(track.id, "id"),
                other => panic!("unexpected command: {other:?}"),
            },
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[tokio::test]
    async fn daemon_frames_round_trip() {
        let reply = round_trip(&DaemonFrame::Reply {
            id: 3,
            result: ReplyResult::Result(ReplyBody::Ack),
        })
        .await;
        assert!(matches!(
            reply,
            DaemonFrame::Reply {
                id: 3,
                result: ReplyResult::Result(ReplyBody::Ack)
            }
        ));
        let error = round_trip(&DaemonFrame::Reply {
            id: 4,
            result: ReplyResult::Error("protocol mismatch".into()),
        })
        .await;
        assert!(matches!(
            error,
            DaemonFrame::Reply { id: 4, result: ReplyResult::Error(message) }
                if message == "protocol mismatch"
        ));
        let event = round_trip(&DaemonFrame::Event {
            event: Box::new(WireEvent::HistoryChanged),
        })
        .await;
        match event {
            DaemonFrame::Event { event } => {
                assert!(matches!(*event, WireEvent::HistoryChanged));
            }
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[tokio::test]
    async fn snapshot_round_trips_from_domain_state() {
        let mut domain = crate::app::state::DomainState::default();
        domain
            .queue
            .push(crate::media::Track::new("a", "Title", "Channel"));
        domain.mpv_ready = true;
        let snapshot = crate::protocol::Snapshot::from(&domain);
        let restored = round_trip(&DaemonFrame::Welcome {
            protocol: 1,
            snapshot: Box::new(snapshot),
        })
        .await;
        match restored {
            DaemonFrame::Welcome { snapshot, .. } => {
                assert_eq!(snapshot.queue.tracks.len(), 1);
                assert!(snapshot.health.mpv_ready);
                assert!(!snapshot.health.yt_dlp_ready);
            }
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[tokio::test]
    async fn eof_at_frame_boundary_is_none() {
        let mut reader = std::io::Cursor::new(Vec::new());
        let frame: Option<ClientFrame> = read_frame(&mut reader).await.expect("eof read");
        assert!(frame.is_none());
    }

    #[tokio::test]
    async fn oversized_frames_are_rejected_on_read() {
        let mut line = vec![b'x'; MAX_FRAME_BYTES + 2];
        line.push(b'\n');
        let mut reader = std::io::Cursor::new(line);
        let result: std::io::Result<Option<ClientFrame>> = read_frame(&mut reader).await;
        assert!(result.is_err(), "oversized frame must fail");
    }

    #[tokio::test]
    async fn malformed_json_is_an_error_not_a_disconnect() {
        let mut reader = std::io::Cursor::new(b"not json\n".to_vec());
        let result: std::io::Result<Option<ClientFrame>> = read_frame(&mut reader).await;
        assert!(result.is_err());
    }
}
