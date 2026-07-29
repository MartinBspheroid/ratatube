//! Audio playback through a persistent mpv process (PRD section 12).

pub mod controller;
pub mod ipc;
pub mod mpv;

pub use controller::PlaybackController;
pub use mpv::MpvProcess;
pub use ratatube_domain::playback::{
    AudioLevels, PREVIOUS_RESTART_THRESHOLD, PlaybackEvent, PlaybackSnapshot, PlaybackStatus,
    SEEK_LARGE, SEEK_SMALL, TRANSITION_DURATION, TrackTransitionState, TransitionInput, events,
    snapshot, transition,
};
