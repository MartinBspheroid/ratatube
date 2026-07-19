//! Background-operation and session-resume state.

use crate::app::operations::OperationId;

/// A track preloaded from the previous session, armed for one-key resume.
#[derive(Debug, Clone)]
pub struct PendingResume {
    pub track: crate::media::Track,
    pub position_seconds: f64,
    /// True once the stream URL arrived and mpv holds the track paused.
    pub armed: bool,
    /// The user already pressed play while resolution was in flight, so start
    /// playback as soon as the stream loads.
    pub play_on_load: bool,
}

/// User-visible lifecycle of a cancellable background operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OperationStatus {
    #[default]
    Idle,
    Loading {
        operation_id: OperationId,
    },
    Failed {
        message: String,
    },
    Cancelled,
}

/// Background metadata lifecycle for the current track.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DetailsStatus {
    #[default]
    Idle,
    Loading {
        operation_id: OperationId,
        track_id: String,
    },
    Failed {
        track_id: String,
        message: String,
    },
}

/// State of the playlist import flow (PRD 10.8).
#[derive(Debug, Clone)]
pub enum ImportState {
    /// Fetching remote metadata.
    Fetching {
        operation_id: OperationId,
        url: String,
    },
    /// Ready for user review before saving.
    Review {
        summary: crate::playlists::import::ImportSummary,
        playlist: Box<crate::playlists::Playlist>,
    },
    /// Fetching terminated with an actionable error.
    Failed { url: String, message: String },
}
