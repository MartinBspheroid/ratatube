//! Application actions: everything that can change state or trigger an
//! effect, per the event-driven architecture in PRD section 13.

use crate::app::operations::OperationId;
use crate::app::state::View;
use crate::media::Track;
use crate::queue::RepeatMode;

/// A state-changing intent, produced by input, timers, or subprocess events.
#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    Navigate(View),
    /// Open help while remembering the current view.
    OpenHelp,
    /// Return from help to the view that opened it.
    CloseHelp,
    /// Scroll the help document by signed rows.
    ScrollHelp(i32),
    NextView,
    PreviousView,
    /// Move Home section focus forward/backward.
    CycleHomeSection(i32),
    Quit,

    // Session resume (instant play)
    /// Stream URL for the previous session's track arrived; load it into
    /// mpv paused (or playing, when requested) at the saved position.
    SessionStreamResolved {
        operation_id: OperationId,
        track_id: String,
        url: String,
    },
    /// Resolution failed; drop the resume card.
    SessionResolveFailed {
        operation_id: OperationId,
        track_id: String,
        message: String,
    },

    // Search
    SearchInput(char),
    SearchBackspace,
    SubmitSearch(String),
    /// Fetch one exact video URL without routing it through search.
    SubmitExactVideo(String),
    SearchCompleted {
        generation: u64,
        tracks: Vec<Track>,
    },
    SearchFailed {
        generation: u64,
        message: String,
    },
    ClearSearch,
    /// Toggle the selected-result detail overlay on narrow Search layouts.
    ToggleSearchDetail,
    /// Open the selected Search result or current Playing track in a browser.
    OpenInBrowser,
    /// Clear the persisted Activity panel.
    ClearActivity,

    // Selection within the active list
    SelectNext,
    SelectPrevious,

    // Playback
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
    /// Seek to a fraction (0.0-1.0) of the duration, e.g. timeline click.
    SeekToFraction(f64),
    VolumeUp,
    VolumeDown,
    ToggleMute,
    ToggleShuffle,
    CycleRepeat,
    PlaybackEvent(crate::playback::PlaybackEvent),
    /// Extended metadata fetch started for the now-playing view.
    DetailsStarted {
        operation_id: OperationId,
        track_id: String,
    },
    /// Extended metadata for the now-playing view arrived (PRD 10.1).
    DetailsLoaded {
        operation_id: OperationId,
        track_id: String,
        details: Box<crate::media::TrackDetails>,
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
    /// Jump to the next chapter of the current track (DJ-mix tracklists).
    NextChapter,
    /// Jump back: chapter start first, then the previous chapter.
    PreviousChapter,
    /// Toggle the Playing view's right pane between chapters/description.
    ToggleNowPlayingPane,
    /// Switch between info and queue focus in the ultra-wide Playing view.
    CyclePlayingPane,

    // Queue
    AddToQueue(Track),
    AddNext(Track),
    RemoveSelectedFromQueue,
    /// Restore the most recently removed queue item.
    UndoQueueRemoval,
    /// Move the selected queue item up (-1) or down (+1) in play order.
    MoveSelectedInQueue(i32),
    ClearQueue,
    /// Clear after explicit confirmation.
    ClearQueueConfirmed,
    QueueLoaded(Track),
    QueueExhausted,

    // Selection-resolving actions (resolved by the app layer, which has
    // access to services and the full state)
    AddSelectedToQueue,
    AddSelectedAsNext,
    LoadSelectedPlaylistIntoQueue,
    AppendSelectedPlaylistToQueue,
    DeleteSelectedPlaylist,

    // Playlists
    OpenPlaylistDetail,
    SaveQueueAsPlaylist(String),
    CreatePlaylist(String),
    RenameSelectedPlaylist(String),
    LoadPlaylistIntoQueue(String),
    AppendPlaylistToQueue(String),
    DeletePlaylist(String),
    /// Actually delete after the user confirmed (PRD 10.7).
    DeletePlaylistConfirmed(String),
    PlaylistSaved(crate::playlists::Playlist),

    // Import (PRD 10.8)
    StartImport(String),
    ImportStarted {
        operation_id: OperationId,
        url: String,
    },
    ImportCompleted {
        operation_id: OperationId,
        url: String,
        title: String,
        remote_id: Option<String>,
        tracks: Vec<Track>,
        rejections: crate::media::yt_dlp::ImportRejections,
    },
    ImportFailed {
        operation_id: OperationId,
        url: String,
        message: String,
    },
    ConfirmImport,
    CancelImport,

    // Modal UI
    OpenPrompt(crate::app::state::PromptPurpose),
    PromptInput(char),
    /// Insert a bracketed-paste payload without treating embedded newlines as submit.
    PromptPaste(String),
    PromptBackspace,
    PromptSubmit,
    PromptCancel,
    /// Open the metadata editor for the active playlist.
    OpenPlaylistEditor,
    PlaylistEditorInput(char),
    PlaylistEditorBackspace,
    PlaylistEditorNextField,
    PlaylistEditorSubmit,
    PlaylistEditorCancel,
    ConfirmYes,
    ConfirmNo,

    // Add-to-playlist picker (P in track lists)
    OpenPlaylistPicker,
    PickerInput(char),
    PickerBackspace,
    PickerNext,
    PickerPrevious,
    /// Resolved by the app layer: adds the track (creating a playlist if
    /// the "new" entry is selected) and closes the picker.
    PickerSubmit,
    PickerCancel,

    // Playlist detail editing
    RemoveSelectedFromPlaylist,
    /// Move the selected playlist track up (-1) or down (+1).
    MoveSelectedInPlaylist(i32),

    // History
    ClearHistory,
    /// Clear after explicit confirmation.
    ClearHistoryConfirmed,
    /// Delete one history entry (Recent mode).
    DeleteSelectedHistoryEntry,
    /// Toggle Recent <-> Top (aggregated) presentation.
    ToggleHistoryViewMode,

    // Playback feel
    /// Adjust playback speed by ±0.25 (clamped 0.5–2.0).
    SpeedUp,
    SpeedDown,
    SpeedReset,
    /// Cycle the sleep timer: off -> 15 -> 30 -> 60 -> off minutes.
    CycleSleepTimer,
    /// Toggle radio mode (auto-refill the queue from YouTube mixes).
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

    // Notifications
    Notify(String),
    DismissNotification,
    /// Toggle the notification log overlay (`!`).
    ToggleNotificationLog,

    // Repeat/shuffle changes coming from elsewhere
    RepeatChanged(RepeatMode),
}
