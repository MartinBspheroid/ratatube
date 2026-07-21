//! Domain half of the application state: playback, queue, playlists,
//! history-adjacent data, and external-tool health. This is the state the
//! future daemon owns; it must stay free of terminal/rendering types.

use crate::app::operations::OperationId;
use crate::app::state::{DetailsStatus, ImportState, OperationStatus, PendingResume, SleepTimer};
use crate::media::Track;
use crate::media::import::InputKind;
use crate::media::search::SearchState;
use crate::playback::{PlaybackSnapshot, TrackTransitionState};
use crate::playlists::Playlist;
use crate::queue::Queue;

/// Domain state: everything the service half owns. Reducers mutate it; the
/// UI half only reads it.
#[derive(Default)]
pub struct DomainState {
    // Search
    pub search: SearchState,
    /// Classified kind of the current search input (URL vs query).
    pub input_kind: Option<InputKind>,
    /// Monotonic generation used to discard superseded searches (PRD 15).
    pub search_generation: u64,

    // Queue
    pub queue: Queue,
    /// Monotonic identity for queue membership and play-order occurrences.
    pub(crate) queue_revision: u64,
    /// Single-level undo for accidental queue deletion.
    pub removed_queue_item: Option<(usize, Track)>,

    // Playlists
    pub playlists: Vec<Playlist>,
    /// Monotonic identity for stored playlist membership and track ordering.
    pub(crate) playlists_revision: u64,

    /// Dedicated channel-browser lifecycle, absent outside that flow.
    pub channel: Option<crate::app::channel::ChannelState>,
    /// Playlist import flow state (fetching, review, failed).
    pub import: Option<ImportState>,

    // Playback
    pub playback: PlaybackSnapshot,
    pub current_track: Option<Track>,
    /// One-shot final-window title transition timing.
    pub track_transition: TrackTransitionState,
    /// Unique generation for each accepted playback load.
    pub(crate) playback_occurrence: u64,
    /// Occurrence for which mpv has emitted the genuine `file-loaded` boundary.
    pub(crate) playback_loaded_occurrence: Option<u64>,
    /// Occurrence owning the current position snapshot.
    pub(crate) position_occurrence: Option<u64>,
    /// Occurrence owning the current duration snapshot.
    pub(crate) duration_occurrence: Option<u64>,
    /// Resolution state for the track requested by the queue cursor.
    pub playback_resolution: OperationStatus,
    /// Extended metadata for the current track, loaded in the background.
    pub current_details: Option<crate::media::TrackDetails>,
    /// Truthful status for metadata shown in the now-playing view.
    pub details_status: DetailsStatus,

    // Radio / timers
    /// Radio mode: when the queue runs low, append tracks from YouTube's mix
    /// for the last played track.
    pub radio: bool,
    /// Active radio refill, used to reject late results after disable.
    pub radio_operation: Option<OperationId>,
    /// Sleep timer that stops playback at its deadline.
    pub sleep_timer: Option<SleepTimer>,

    // Session / history-adjacent
    /// Previous-session track preloaded for one-key resume.
    pub pending_resume: Option<PendingResume>,
    /// Persisted, bounded product activity shown on Home and Playing.
    pub activity: crate::history::activity::ActivityLog,
    /// Persisted per-track resume positions shown on Home.
    pub resume_points: crate::persistence::resume::ResumePoints,

    // External-tool health
    pub mpv_ready: bool,
    pub yt_dlp_ready: bool,
}

impl DomainState {
    /// Invalidate queue occurrence tokens after a membership or order change.
    pub(crate) fn bump_queue_revision(&mut self) {
        self.queue_revision = self.queue_revision.wrapping_add(1);
    }

    /// Invalidate playlist occurrence tokens after a stored collection change.
    pub(crate) fn bump_playlists_revision(&mut self) {
        self.playlists_revision = self.playlists_revision.wrapping_add(1);
    }
}
