//! Central application state (PRD section 13).

use crate::media::Track;
use crate::media::import::InputKind;
use crate::media::search::SearchState;
use crate::playback::PlaybackSnapshot;
use crate::playlists::Playlist;
use crate::queue::Queue;

/// Primary views (PRD section 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
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
    pub const TABS: [View; 5] = [
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

/// A transient user-facing notification (PRD section 16).
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub is_error: bool,
}

/// Which pane currently holds keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// The search input field is capturing text.
    SearchInput,
    /// The main content list.
    #[default]
    Content,
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
    /// Name for a new empty playlist.
    NewPlaylist,
}

/// Active single-line prompt state.
#[derive(Debug, Clone)]
pub struct PromptState {
    pub purpose: PromptPurpose,
    pub buffer: String,
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
    Fetching { url: String },
    /// Ready for user review before saving.
    Review {
        summary: crate::playlists::import::ImportSummary,
        playlist: Box<crate::playlists::Playlist>,
    },
}

/// Root application state. UI renders this; reducers mutate it.
#[derive(Default)]
pub struct AppState {
    pub running: bool,
    pub view: View,
    pub focus: Focus,

    // Search
    pub search: SearchState,
    pub search_input: String,
    /// Classified kind of the current search input (URL vs query).
    pub input_kind: Option<InputKind>,

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
    /// Last rendered full screen area, for mouse hit-testing.
    pub screen_area: ratatui::layout::Rect,
    pub queue: Queue,
    pub playlists: Vec<Playlist>,
    pub selected_playlist: Option<usize>,

    // Modal UI
    pub prompt: Option<PromptState>,
    pub confirm: Option<ConfirmState>,
    pub import: Option<ImportState>,

    // Playback
    pub playback: PlaybackSnapshot,
    pub current_track: Option<Track>,
    /// Extended metadata for the current track, loaded in the background.
    pub current_details: Option<crate::media::TrackDetails>,
    /// Decoded thumbnail for the current track, when available.
    pub thumbnail: Option<ratatui_image::protocol::StatefulProtocol>,
    /// Scroll offset of the now-playing description panel.
    pub now_playing_scroll: u16,

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

    /// Length of the list backing the current view.
    pub fn active_list_len(&self) -> usize {
        match self.view {
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
            View::History => 0, // Populated from HistoryService at render time.
            View::NowPlaying | View::Help => 0,
        }
    }
}
