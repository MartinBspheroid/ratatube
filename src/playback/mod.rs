//! Audio playback through a persistent mpv process (PRD section 12).

pub mod controller;
pub mod events;
pub mod ipc;
pub mod mpv;
pub mod transition;
#[cfg(test)]
mod transition_tests;

pub use controller::{PlaybackController, PlaybackSnapshot};
pub use events::{AudioLevels, PlaybackEvent, PlaybackStatus};
pub use mpv::MpvProcess;
pub use transition::{TRANSITION_DURATION, TrackTransitionState, TransitionInput};
