//! Central application state (PRD section 13).

use crate::app::operations::OperationId;
use crate::media::Track;
use crate::media::import::InputKind;
use crate::media::search::SearchState;
use crate::playback::PlaybackSnapshot;
use crate::playlists::Playlist;
use crate::queue::Queue;

/// Primary views (PRD section 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// Landing dashboard: resume card, recent tracks, playlists.
    #[default]
    Home,
    Search,
    Queue,
    Playlists,
    PlaylistDetail,
    History,
    NowPlaying,
    Help,
}

impl View {
    /// Display label for the header.
    pub fn title(self) -> &'static str {
        match self {
            View::Home => "Home",
            View::Search => "Search",
            View::Queue => "Queue",
            View::Playlists => "Playlists",
            View::PlaylistDetail => "Playlist",
            View::History => "History",
            View::NowPlaying => "Now Playing",
            View::Help => "Help",
        }
    }

    /// Tab order shown in the header (excludes detail/help views).
    pub const TABS: [View; 6] = [
        View::Home,
        View::Search,
        View::Queue,
        View::Playlists,
        View::History,
        View::NowPlaying,
    ];

    /// Next view in tab order, wrapping around.
    pub fn next_tab(self) -> View {
        let pos = View::TABS.iter().position(|v| *v == self).unwrap_or(0);
        View::TABS[(pos + 1) % View::TABS.len()]
    }

    /// Previous view in tab order, wrapping around.
    pub fn prev_tab(self) -> View {
        let pos = View::TABS.iter().position(|v| *v == self).unwrap_or(0);
        View::TABS[(pos + View::TABS.len() - 1) % View::TABS.len()]
    }
}

/// Focusable sections of the Home dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HomeSection {
    #[default]
    Resume,
    Recent,
    Playlists,
}

/// Keyboard focus within the ultra-wide Playing view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayingPane {
    #[default]
    Info,
    Queue,
}

impl HomeSection {
    /// Cycle focus forward (+1) or backward (-1).
    pub fn cycled(self, delta: i32) -> HomeSection {
        const ORDER: [HomeSection; 3] = [
            HomeSection::Resume,
            HomeSection::Recent,
            HomeSection::Playlists,
        ];
        let pos = ORDER.iter().position(|s| *s == self).unwrap_or(0) as i32;
        let next = (pos + delta).rem_euclid(ORDER.len() as i32);
        ORDER[next as usize]
    }
}

/// A track preloaded from the previous session, armed for one-key resume.
#[derive(Debug, Clone)]
pub struct PendingResume {
    pub track: crate::media::Track,
    pub position_seconds: f64,
    /// True once the stream URL arrived and mpv holds the track paused.
    pub armed: bool,
    /// The user already pressed play while we were still resolving; start
    /// playback as soon as the stream loads.
    pub play_on_load: bool,
}

/// A transient user-facing notification (PRD section 16).
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub is_error: bool,
    pub created_at: chrono::DateTime<chrono::Local>,
    expires_at: std::time::Instant,
}

impl Notification {
    /// Build a notification with a deterministic creation instant.
    pub fn new_at(message: &str, is_error: bool, now: std::time::Instant) -> Self {
        let lifetime = if is_error {
            std::time::Duration::from_secs(8)
        } else {
            std::time::Duration::from_secs(4)
        };
        Self {
            message: message.to_string(),
            is_error,
            created_at: chrono::Local::now(),
            expires_at: now + lifetime,
        }
    }

    /// Whether this transient message has reached its deadline.
    pub fn is_expired_at(&self, now: std::time::Instant) -> bool {
        now >= self.expires_at
    }
}

/// An armed sleep timer.
#[derive(Debug, Clone, Copy)]
pub struct SleepTimer {
    pub deadline: std::time::Instant,
    /// The duration originally chosen, for cycling and display.
    pub minutes: u16,
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

impl SleepTimer {
    /// Whole minutes left, rounded up; 0 when due.
    pub fn remaining_minutes(&self) -> u64 {
        let now = std::time::Instant::now();
        self.deadline
            .saturating_duration_since(now)
            .as_secs()
            .div_ceil(60)
    }
}

/// Which pane currently holds keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// The search input field is capturing text.
    SearchInput,
    /// The in-list filter bar is capturing text (`/` in list views).
    ListFilter,
    /// The main content list.
    #[default]
    Content,
}

/// How the History view presents its entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryViewMode {
    /// Chronological log, newest first.
    #[default]
    Recent,
    /// Aggregated per track: play counts and total listened time.
    Top,
}

/// State of the add-to-playlist picker modal.
#[derive(Debug, Clone)]
pub struct PickerState {
    /// Track(s) to add on submit.
    pub track: crate::media::Track,
    /// Typed filter over playlist names; a non-matching name becomes a
    /// "create new playlist" entry.
    pub filter: String,
    /// Selection within the visible candidate list (0 may be "create new").
    pub selected: usize,
}

/// Purpose of a single-line text prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptPurpose {
    /// Name for saving the current queue as a playlist.
    SaveQueueAsPlaylist,
    /// Rename the selected playlist.
    RenamePlaylist,
    /// URL for importing a remote playlist.
    ImportPlaylistUrl,
    /// Versioned JSON containing one or more local playlists.
    ImportPlaylistJson,
    /// Name for a new empty playlist.
    NewPlaylist,
}

/// Active text prompt state. JSON imports may contain pasted newlines.
#[derive(Debug, Clone)]
pub struct PromptState {
    pub purpose: PromptPurpose,
    pub buffer: String,
}

/// Active field in the playlist metadata editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistEditorField {
    Name,
    Description,
}

/// Draft playlist metadata; persisted only when explicitly submitted.
#[derive(Debug, Clone)]
pub struct PlaylistEditorState {
    pub name: String,
    pub description: String,
    pub field: PlaylistEditorField,
}

/// A yes/no confirmation dialog (e.g. playlist deletion, PRD 10.7).
#[derive(Debug, Clone)]
pub struct ConfirmState {
    pub message: String,
    pub action: Box<crate::app::action::Action>,
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
    /// Scroll/selection state for the active list; reset on navigation.
    /// Kept outside render calls so ratatui can scroll naturally.
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
    /// Single-level undo for accidental queue deletion.
    pub removed_queue_item: Option<(usize, Track)>,
    pub playlists: Vec<Playlist>,
    pub selected_playlist: Option<usize>,

    // Modal UI
    pub prompt: Option<PromptState>,
    pub playlist_editor: Option<PlaylistEditorState>,
    pub confirm: Option<ConfirmState>,
    pub import: Option<ImportState>,
    /// Add-to-playlist picker (`P` in track lists).
    pub picker: Option<PickerState>,

    // In-list filtering (`/` in list views)
    /// Active filter text; `None` when no filter is applied.
    pub list_filter: Option<String>,
    /// Indices of the underlying list that pass the filter, when one is
    /// active. Kept in sync by the app layer each loop iteration.
    pub visible_indices: Option<Vec<usize>>,

    /// Presentation mode of the History view.
    pub history_view_mode: HistoryViewMode,

    /// Radio mode: when the queue runs low, append tracks from YouTube's
    /// mix for the last played track.
    pub radio: bool,
    /// Active radio refill, used to reject late results after disable.
    pub radio_operation: Option<OperationId>,
    /// Sleep timer: stop playback at the deadline.
    pub sleep_timer: Option<SleepTimer>,
    /// Recent notifications, newest first (bounded ring).
    pub notification_log: std::collections::VecDeque<Notification>,
    /// Whether the notification log overlay is open.
    pub show_notification_log: bool,

    // Playback
    pub playback: PlaybackSnapshot,
    pub current_track: Option<Track>,
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
    /// Show the description instead of chapters in the Playing view's
    /// right pane (only meaningful when chapters exist).
    pub now_playing_show_description: bool,
    /// Active pane when the ultra-wide Playing layout exposes its queue.
    pub playing_pane: PlayingPane,

    // Home dashboard
    /// Which Home section holds the selection.
    pub home_section: HomeSection,
    /// Number of deduplicated recent tracks shown on Home (set at render).
    pub home_recent_len: usize,
    /// Previous-session track preloaded for one-key resume.
    pub pending_resume: Option<PendingResume>,
    /// Persisted, bounded product activity shown on Home and Playing.
    pub activity: crate::history::activity::ActivityLog,
    /// Persisted per-track resume positions shown on Home.
    pub resume_points: crate::persistence::resume::ResumePoints,

    /// History entry count, mirrored from the service each loop iteration so
    /// selection movement works (the state cannot see the service itself).
    pub history_len: usize,

    // Status
    pub mpv_ready: bool,
    pub yt_dlp_ready: bool,
    pub notification: Option<Notification>,

    /// Resolved icon mode from configuration (PRD 10.12).
    pub icon_mode: crate::config::IconMode,

    /// Monotonic generation so superseded searches are discarded (PRD 15).
    pub search_generation: u64,
}

impl AppState {
    /// Keep playlist presentation and selection order newest-updated first.
    pub fn sort_playlists_by_updated(&mut self) {
        self.playlists
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }

    pub fn new() -> Self {
        Self {
            running: true,
            ..Self::default()
        }
    }

    /// Attach loaded services' initial data (queue restored from disk, etc.).
    pub fn with_queue(mut self, queue: Queue) -> Self {
        self.queue = queue;
        self
    }

    /// Whether the now-playing bar should render (PRD section 8).
    pub fn has_now_playing(&self) -> bool {
        self.current_track.is_some()
    }

    /// Chapters of the current track (uploader-set or parsed tracklist).
    pub fn chapters(&self) -> &[crate::media::Chapter] {
        self.current_details
            .as_ref()
            .map_or(&[], |d| d.chapters.as_slice())
    }

    /// Index of the chapter the playhead is currently inside.
    pub fn current_chapter_index(&self) -> Option<usize> {
        crate::media::chapter_at(self.chapters(), self.playback.position_seconds)
    }

    /// Clamp selection to the length of the active list.
    pub fn clamp_selection(&mut self) {
        let len = self.active_list_len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    /// Reset list scroll state; called on navigation between views.
    pub fn reset_list(&mut self) {
        self.list_state = ratatui::widgets::ListState::default();
        self.table_state = ratatui::widgets::TableState::default();
        self.clamp_selection();
    }

    /// Advance the spinner animation frame.
    pub fn tick_spinner(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    /// Map a position in the (possibly filtered) visible list back to an
    /// index into the underlying list.
    pub fn resolve_index(&self, visible: usize) -> usize {
        match &self.visible_indices {
            Some(indices) => indices.get(visible).copied().unwrap_or(visible),
            None => visible,
        }
    }

    /// Length of the list backing the current view.
    pub fn active_list_len(&self) -> usize {
        if let Some(indices) = &self.visible_indices {
            return indices.len();
        }
        match self.view {
            View::Home => match self.home_section {
                HomeSection::Resume => 0,
                HomeSection::Recent => self.home_recent_len,
                HomeSection::Playlists => self.playlists.len(),
            },
            View::Search => match &self.search {
                SearchState::Results { tracks, .. } => tracks.len(),
                _ => 0,
            },
            View::Queue => self.queue.order.len(),
            View::Playlists => self.playlists.len(),
            View::PlaylistDetail => self
                .selected_playlist
                .and_then(|i| self.playlists.get(i))
                .map_or(0, |p| p.tracks.len()),
            View::History => self.history_len,
            View::NowPlaying
                if self.playing_pane == PlayingPane::Queue
                    && crate::ui::layout::Breakpoint::from_width(self.screen_area.width)
                        == crate::ui::layout::Breakpoint::UltraWide =>
            {
                self.queue.order.len()
            }
            View::NowPlaying | View::Help => 0,
        }
    }
}

#[cfg(test)]
mod home_section_tests {
    use super::HomeSection;

    #[test]
    fn home_section_ring_follows_visual_reading_order() {
        let mut section = HomeSection::Resume;
        let mut visited = Vec::new();
        for _ in 0..3 {
            visited.push(section);
            section = section.cycled(1);
        }
        assert_eq!(
            visited,
            vec![
                HomeSection::Resume,
                HomeSection::Recent,
                HomeSection::Playlists,
            ]
        );
        assert_eq!(section, HomeSection::Resume);
        assert_eq!(HomeSection::Resume.cycled(-1), HomeSection::Playlists);
    }
}
