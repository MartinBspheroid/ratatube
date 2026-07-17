//! Audio playback through a persistent mpv process (PRD section 12).

pub mod controller;
pub mod events;
pub mod ipc;
pub mod mpv;

pub use controller::{PlaybackController, PlaybackSnapshot};
pub use events::{PlaybackEvent, PlaybackStatus};
pub use mpv::MpvProcess;
