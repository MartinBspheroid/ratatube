//! Application actions: everything that can change state or trigger an
//! effect, per the event-driven architecture in PRD section 13.

use crate::app::state::View;
use crate::media::Track;
use crate::queue::RepeatMode;

/// A state-changing intent, produced by input, timers, or subprocess events.
#[derive(Debug, Clone)]
pub enum Action {
    // Navigation
    Navigate(View),
    NextView,
    PreviousView,
    /// Move Home section focus forward/backward.
    CycleHomeSection(i32),
    Quit,

    // Session resume (instant play)
    /// Stream URL for the previous session's track arrived; load it into
    /// mpv paused (or playing, when requested) at the saved position.
    SessionStreamResolved {
        track_id: String,
        url: String,
    },
    /// Resolution failed; drop the resume card.
    SessionResolveFailed {
        track_id: String,
        message: String,
    },

    // Search
    SearchInput(char),
    SearchBackspace,
    SubmitSearch(String),
    SearchCompleted {
        generation: u64,
        tracks: Vec<Track>,
    },
    SearchFailed {
        generation: u64,
        message: String,
    },
    ClearSearch,

    // Selection within the active list
    SelectNext,
    SelectPrevious,

    // Playback
    PlayPause,
    Stop,
    PlaySelected,
    PlayTrack(Track),
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
    /// Extended metadata for the now-playing view arrived (PRD 10.1).
    DetailsLoaded {
        track_id: String,
        details: Box<crate::media::TrackDetails>,
    },
    /// Thumbnail image bytes for the current track arrived.
    ThumbnailLoaded {
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

    // Queue
    AddToQueue(Track),
    AddNext(Track),
    RemoveSelectedFromQueue,
    /// Move the selected queue item up (-1) or down (+1) in play order.
    MoveSelectedInQueue(i32),
    ClearQueue,
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
    ImportCompleted {
        url: String,
        title: String,
        remote_id: Option<String>,
        tracks: Vec<Track>,
        skipped: usize,
    },
    ImportFailed {
        url: String,
        message: String,
    },
    ConfirmImport,
    CancelImport,

    // Modal UI
    OpenPrompt(crate::app::state::PromptPurpose),
    PromptInput(char),
    PromptBackspace,
    PromptSubmit,
    PromptCancel,
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
        track_id: String,
        url: String,
    },
    /// Radio refill fetched more tracks for the queue.
    RadioTracksLoaded {
        tracks: Vec<Track>,
    },
    /// A pasted mix/radio URL was fetched; replace the queue and play.
    MixLoaded {
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
