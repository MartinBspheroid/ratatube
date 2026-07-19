//! Playback, media-detail, thumbnail, and playback-feel actions.

use crate::app::operations::OperationId;
use crate::media::{Track, TrackDetails};
use crate::playback::PlaybackEvent;
use crate::queue::RepeatMode;

/// An intent that controls playback or reports playback-related background work.
#[derive(Debug, Clone)]
pub enum PlaybackAction {
    /// The previous session's stream URL arrived for paused or immediate resume.
    SessionStreamResolved {
        operation_id: OperationId,
        track_id: String,
        url: String,
    },
    /// Previous-session stream resolution failed.
    SessionResolveFailed {
        operation_id: OperationId,
        track_id: String,
        message: String,
    },
    PlayPause,
    Stop,
    PlaySelected,
    PlayTrack(Track),
    /// Resume a known track at a persisted per-track position.
    ResumeTrack {
        track: Track,
        position_seconds: f64,
    },
    /// Stream resolution started outside the event loop.
    PlaybackResolveStarted {
        operation_id: OperationId,
        queue_position: usize,
        track_id: String,
    },
    /// The active stream resolution produced a playable URL.
    PlaybackResolved {
        operation_id: OperationId,
        queue_position: usize,
        track_id: String,
        url: String,
    },
    /// The active stream resolution exhausted its retry budget.
    PlaybackResolveFailed {
        operation_id: OperationId,
        queue_position: usize,
        track_id: String,
        message: String,
    },
    NextTrack,
    PreviousTrack,
    SeekForward,
    SeekBackward,
    SeekForwardLarge,
    SeekBackwardLarge,
    /// Seek to a fraction of the duration.
    SeekToFraction(f64),
    VolumeUp,
    VolumeDown,
    ToggleMute,
    ToggleShuffle,
    CycleRepeat,
    PlaybackEvent(PlaybackEvent),
    /// Extended metadata fetch started for the now-playing view.
    DetailsStarted {
        operation_id: OperationId,
        track_id: String,
    },
    /// Extended metadata for the now-playing view arrived.
    DetailsLoaded {
        operation_id: OperationId,
        track_id: String,
        details: Box<TrackDetails>,
    },
    /// Extended metadata could not be fetched.
    DetailsFailed {
        operation_id: OperationId,
        track_id: String,
        message: String,
    },
    /// Thumbnail image bytes for the current track arrived.
    ThumbnailLoaded {
        operation_id: OperationId,
        track_id: String,
        bytes: Vec<u8>,
    },
    /// Thumbnail image bytes for the selected Search result arrived.
    SearchThumbnailLoaded {
        operation_id: OperationId,
        track_id: String,
        bytes: Vec<u8>,
    },
    /// Scroll the now-playing description panel.
    ScrollNowPlaying(i32),
    NextChapter,
    PreviousChapter,
    ToggleNowPlayingPane,
    CyclePlayingPane,
    /// Adjust playback speed by a fixed step.
    SpeedUp,
    SpeedDown,
    SpeedReset,
    /// Cycle the sleep timer through its supported durations.
    CycleSleepTimer,
    /// Toggle radio mode for automatic queue refills.
    ToggleRadio,
    /// A background prefetch of the next track's stream URL finished.
    PrefetchResolved {
        operation_id: OperationId,
        track_id: String,
        url: String,
    },
    /// A radio refill started and superseded any prior refill.
    RadioRefillStarted {
        operation_id: OperationId,
    },
    /// Radio refill fetched more tracks for the queue.
    RadioTracksLoaded {
        operation_id: OperationId,
        tracks: Vec<Track>,
    },
    /// A pasted mix/radio URL was fetched; replace the queue and play.
    MixLoaded {
        operation_id: OperationId,
        title: String,
        tracks: Vec<Track>,
    },
    /// Repeat mode changed outside the reducer.
    RepeatChanged(RepeatMode),
}
