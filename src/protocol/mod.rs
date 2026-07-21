//! Daemon control protocol: newline-delimited JSON frames over a Unix
//! socket, mirroring the request-ID discipline of the in-repo mpv IPC
//! client (see `docs/superpowers/specs/2026-07-21-daemon-split-design.md`).

mod codec;
mod frames;

pub use codec::{MAX_FRAME_BYTES, read_frame, write_frame};
pub use frames::{
    ClientFrame, Command, DaemonFrame, Health, ReplyBody, ReplyResult, Snapshot, WireEvent,
    WireImport,
};

/// Version spoken by both halves of this binary. A daemon refuses clients
/// with a different version by replying with an error and disconnecting.
pub const PROTOCOL_VERSION: u32 = 1;
