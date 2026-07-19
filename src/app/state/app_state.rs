//! Root application state consumed by rendering and reducers.

use crate::app::operations::OperationId;
use crate::app::state::{
    ConfirmState, DetailsStatus, Focus, HistoryViewMode, HomeSection, ImportState, Notification,
    OperationStatus, PendingResume, PickerState, PlayingPane, PlaylistEditorState, PromptState,
    SleepTimer, TrackContextMenuState, TrackDetailsModalState, View,
};
use crate::media::Track;
use crate::media::import::InputKind;
use crate::media::search::SearchState;
use crate::playback::{PlaybackSnapshot, TrackTransitionState};
use crate::playlists::Playlist;
use crate::queue::Queue;

/// Root application state. UI renders this; reducers mutate it.
#[derive(Default)]
pub struct AppState {
    pub running: bool,
    pub view: View,
    /// View restored when the help overlay closes.
    pub help_return_view: View,
    /// Vertical row offset in the help document.
    pub help_scroll: u16,
    pub focus: Focus,

    // Search
    pub search: SearchState,
    pub search_input: String,
    /// Classified kind of the current search input (URL vs query).
    pub input_kind: Option<InputKind>,
    /// Narrow Search modal showing details for the highlighted result.
    pub search_detail_open: bool,
    /// Track whose selected-result thumbnail is loaded or being fetched.
    pub search_thumbnail_track_id: Option<String>,
    /// Decoded thumbnail preview for the selected Search result.
    pub search_thumbnail: Option<ratatui_image::protocol::StatefulProtocol>,

    // Lists and selection
    pub selected_index: usize,
    /// Scroll and selection state for the active list. It is reset on
    /// navigation and kept outside render calls so ratatui can scroll naturally.
    pub list_state: ratatui::widgets::ListState,
    /// Selection state for the search results table.
    pub table_state: ratatui::widgets::TableState,
    /// Rotates on every render tick to animate spinners.
    pub spinner_frame: usize,
    /// Last rendered main content area, for mouse hit-testing.
    pub main_area: ratatui::layout::Rect,
    /// Renderer-provided rows that map one-to-one to selectable items.
    pub list_hit_area: ratatui::layout::Rect,
    /// Last rendered full screen area, for mouse hit-testing.
    pub screen_area: ratatui::layout::Rect,
    pub queue: Queue,
    /// Monotonic identity for queue membership and play-order occurrences.
    pub(crate) queue_revision: u64,
    /// Single-level undo for accidental queue deletion.
    pub removed_queue_item: Option<(usize, Track)>,
    pub playlists: Vec<Playlist>,
    /// Monotonic identity for stored playlist membership and track ordering.
    pub(crate) playlists_revision: u64,
    pub selected_playlist: Option<usize>,
    /// Dedicated channel-browser lifecycle, absent outside that flow.
    pub channel: Option<crate::app::channel::ChannelState>,

    // Modal UI
    pub prompt: Option<PromptState>,
    pub playlist_editor: Option<PlaylistEditorState>,
    pub confirm: Option<ConfirmState>,
    pub import: Option<ImportState>,
    /// Add-to-playlist picker (`P` in track lists).
    pub picker: Option<PickerState>,
    /// Universal context actions for the currently resolved track.
    pub track_context_menu: Option<TrackContextMenuState>,
    /// Unique ownership generation for each successfully opened track menu.
    pub(crate) track_context_generation: u64,
    /// Selected-track details that do not replace now-playing metadata.
    pub track_details_modal: Option<TrackDetailsModalState>,

    // In-list filtering (`/` in list views)
    /// Active filter text; `None` when no filter is applied.
    pub list_filter: Option<String>,
    /// Indices of the underlying list that pass the filter. The app layer
    /// keeps this derived mapping synchronized before each render.
    pub visible_indices: Option<Vec<usize>>,
    /// Presentation mode of the History view.
    pub history_view_mode: HistoryViewMode,
    /// Radio mode: when the queue runs low, append tracks from YouTube's mix
    /// for the last played track.
    pub radio: bool,
    /// Active radio refill, used to reject late results after disable.
    pub radio_operation: Option<OperationId>,
    /// Sleep timer that stops playback at its deadline.
    pub sleep_timer: Option<SleepTimer>,
    /// Recent notifications, newest first, in a bounded ring.
    pub notification_log: std::collections::VecDeque<Notification>,
    /// Whether the notification log overlay is open.
    pub show_notification_log: bool,

    // Playback
    pub playback: PlaybackSnapshot,
    pub current_track: Option<Track>,
    /// One-shot final-window title transition timing.
    pub track_transition: TrackTransitionState,
    /// Resolution state for the track requested by the queue cursor.
    pub playback_resolution: OperationStatus,
    /// Extended metadata for the current track, loaded in the background.
    pub current_details: Option<crate::media::TrackDetails>,
    /// Truthful status for metadata shown in the now-playing view.
    pub details_status: DetailsStatus,
    /// Decoded thumbnail for the current track, when available.
    pub thumbnail: Option<ratatui_image::protocol::StatefulProtocol>,
    /// Scroll offset of the now-playing description panel.
    pub now_playing_scroll: u16,
    /// Show the description instead of chapters in the Playing view's right
    /// pane; this is meaningful only when chapters exist.
    pub now_playing_show_description: bool,
    /// Active pane when the ultra-wide Playing layout exposes its queue.
    pub playing_pane: PlayingPane,

    // Home dashboard
    /// Which Home section holds the selection.
    pub home_section: HomeSection,
    /// Number of deduplicated recent tracks shown on Home, set during render.
    pub home_recent_len: usize,
    /// Previous-session track preloaded for one-key resume.
    pub pending_resume: Option<PendingResume>,
    /// Persisted, bounded product activity shown on Home and Playing.
    pub activity: crate::history::activity::ActivityLog,
    /// Persisted per-track resume positions shown on Home.
    pub resume_points: crate::persistence::resume::ResumePoints,
    /// History entry count mirrored from the service before each render so
    /// selection movement works without giving state access to the service.
    pub history_len: usize,

    // Status
    pub mpv_ready: bool,
    pub yt_dlp_ready: bool,
    pub notification: Option<Notification>,
    /// Resolved icon mode from configuration (PRD 10.12).
    pub icon_mode: crate::config::IconMode,
    /// Monotonic generation used to discard superseded searches (PRD 15).
    pub search_generation: u64,
}

impl AppState {
    /// Construct running application state with default data.
    pub fn new() -> Self {
        Self {
            running: true,
            ..Self::default()
        }
    }

    /// Attach loaded services' initial data, including a queue restored from disk.
    pub fn with_queue(mut self, queue: Queue) -> Self {
        self.queue = queue;
        self
    }

    /// Invalidate queue occurrence tokens after a membership or order change.
    pub(crate) fn bump_queue_revision(&mut self) {
        self.queue_revision = self.queue_revision.wrapping_add(1);
    }

    /// Invalidate playlist occurrence tokens after a stored collection change.
    pub(crate) fn bump_playlists_revision(&mut self) {
        self.playlists_revision = self.playlists_revision.wrapping_add(1);
    }
}
