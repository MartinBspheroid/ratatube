//! Playback vocabulary: mpv-shaped events, the observable snapshot, and the
//! final-window track transition.

pub mod events;
pub mod snapshot;
pub mod transition;
#[cfg(test)]
mod transition_tests;

pub use events::{AudioLevels, PlaybackEvent, PlaybackStatus};
pub use snapshot::{PREVIOUS_RESTART_THRESHOLD, PlaybackSnapshot, SEEK_LARGE, SEEK_SMALL};
pub use transition::{TRANSITION_DURATION, TrackTransitionState, TransitionInput};
