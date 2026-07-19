//! Root application state consumed by rendering and reducers.

use crate::app::operations::OperationId;
use crate::app::state::{
    ConfirmState, DetailsStatus, Focus, HistoryViewMode, HomeSection, ImportState, Notification,
    OperationStatus, PendingResume, PickerState, PlayingPane, PlaylistEditorState, PromptState,
    SleepTimer, View,
};
use crate::media::Track;
use crate::media::import::InputKind;
use crate::media::search::SearchState;
use crate::playback::PlaybackSnapshot;
use crate::playlists::Playlist;
use crate::queue::Queue;

/// Root application state. UI renders this; reducers mutate it.
#[derive(Default)]
pub struct AppState {
    pub running: bool,
    pub view: View,
    pub help_return_view: View,
    pub help_scroll: u16,
    pub focus: Focus,
    pub search: SearchState,
    pub search_input: String,
    pub input_kind: Option<InputKind>,
    pub search_detail_open: bool,
    pub search_thumbnail_track_id: Option<String>,
    pub search_thumbnail: Option<ratatui_image::protocol::StatefulProtocol>,
    pub selected_index: usize,
    pub list_state: ratatui::widgets::ListState,
    pub table_state: ratatui::widgets::TableState,
    pub spinner_frame: usize,
    pub main_area: ratatui::layout::Rect,
    pub list_hit_area: ratatui::layout::Rect,
    pub screen_area: ratatui::layout::Rect,
    pub queue: Queue,
    pub removed_queue_item: Option<(usize, Track)>,
    pub playlists: Vec<Playlist>,
    pub selected_playlist: Option<usize>,
    pub prompt: Option<PromptState>,
    pub playlist_editor: Option<PlaylistEditorState>,
    pub confirm: Option<ConfirmState>,
    pub import: Option<ImportState>,
    pub picker: Option<PickerState>,
    pub list_filter: Option<String>,
    pub visible_indices: Option<Vec<usize>>,
    pub history_view_mode: HistoryViewMode,
    pub radio: bool,
    pub radio_operation: Option<OperationId>,
    pub sleep_timer: Option<SleepTimer>,
    pub notification_log: std::collections::VecDeque<Notification>,
    pub show_notification_log: bool,
    pub playback: PlaybackSnapshot,
    pub current_track: Option<Track>,
    pub playback_resolution: OperationStatus,
    pub current_details: Option<crate::media::TrackDetails>,
    pub details_status: DetailsStatus,
    pub thumbnail: Option<ratatui_image::protocol::StatefulProtocol>,
    pub now_playing_scroll: u16,
    pub now_playing_show_description: bool,
    pub playing_pane: PlayingPane,
    pub home_section: HomeSection,
    pub home_recent_len: usize,
    pub pending_resume: Option<PendingResume>,
    pub activity: crate::history::activity::ActivityLog,
    pub resume_points: crate::persistence::resume::ResumePoints,
    pub history_len: usize,
    pub mpv_ready: bool,
    pub yt_dlp_ready: bool,
    pub notification: Option<Notification>,
    pub icon_mode: crate::config::IconMode,
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

    /// Attach loaded services' initial queue.
    pub fn with_queue(mut self, queue: Queue) -> Self {
        self.queue = queue;
        self
    }
}
