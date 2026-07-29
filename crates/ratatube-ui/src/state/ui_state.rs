//! UI half of the application state: navigation, selection, overlays,
//! filters, and presentation-only data. This is the state the future client
//! keeps locally; the daemon never sees it.

use crate::state::{
    ConfirmState, Focus, HistoryViewMode, HomeSection, Notification, PickerState, PlayingPane,
    PlaylistEditorState, PromptState, SettingsState, TrackContextMenuState, TrackDetailsModalState,
    View,
};

/// One clickable Home item: clicking it focuses its section and selects
/// `index`; the Resume card registers a single zone with index 0.
#[derive(Debug, Clone, Copy)]
pub struct HomeHitZone {
    pub section: HomeSection,
    pub index: usize,
    pub area: ratatui::layout::Rect,
}

/// UI state: everything rendering and input own. Never read by the domain
/// half; domain changes reach it only through `apply_domain_events`.
#[derive(Default)]
pub struct UiState {
    pub running: bool,
    pub view: View,
    /// View restored when the help overlay closes.
    pub help_return_view: View,
    /// Vertical row offset in the help document.
    pub help_scroll: u16,
    pub focus: Focus,

    // Search presentation
    pub search_input: String,
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
    pub(crate) spinner_frame: usize,
    /// Last rendered main content area, for mouse hit-testing.
    pub main_area: ratatui::layout::Rect,
    /// Renderer-provided rows that map one-to-one to selectable items.
    pub list_hit_area: ratatui::layout::Rect,
    /// Index of the first visible item in `list_hit_area` for views that
    /// window rows manually; `None` means the list/table state's offset.
    pub list_hit_offset: Option<usize>,
    /// Clickable Home items registered per render; the dashboard's sections
    /// have no shared list geometry, so every item carries its own area.
    pub home_hit_zones: Vec<HomeHitZone>,
    /// Last rendered full screen area, for mouse hit-testing.
    pub screen_area: ratatui::layout::Rect,
    pub selected_playlist: Option<usize>,

    // Modal UI
    pub prompt: Option<PromptState>,
    pub playlist_editor: Option<PlaylistEditorState>,
    pub confirm: Option<ConfirmState>,
    /// Add-to-playlist picker (`P` in track lists).
    pub picker: Option<PickerState>,
    /// Universal context actions for the currently resolved track.
    pub track_context_menu: Option<TrackContextMenuState>,
    /// Unique ownership generation for each successfully opened track menu.
    pub track_context_generation: u64,
    /// Selected-track details that do not replace now-playing metadata.
    pub track_details_modal: Option<TrackDetailsModalState>,
    /// The ctrl+p settings menu.
    pub settings: Option<SettingsState>,

    // In-list filtering (`/` in list views)
    /// Active filter text; `None` when no filter is applied.
    pub list_filter: Option<String>,
    /// Indices of the underlying list that pass the filter. The app layer
    /// keeps this derived mapping synchronized before each render.
    pub visible_indices: Option<Vec<usize>>,
    /// Presentation mode of the History view.
    pub history_view_mode: HistoryViewMode,

    // Notifications
    /// Recent notifications, newest first, in a bounded ring.
    pub(crate) notification_log: std::collections::VecDeque<Notification>,
    /// Whether the notification log overlay is open.
    pub show_notification_log: bool,
    pub notification: Option<Notification>,

    // Now-playing presentation
    /// Decoded thumbnail for the current track, when available.
    pub thumbnail: Option<ratatui_image::protocol::StatefulProtocol>,
    /// Scroll offset of the now-playing description panel.
    pub now_playing_scroll: u16,
    /// Show the description instead of chapters in the Playing view's right
    /// pane; this is meaningful only when chapters exist.
    pub(crate) now_playing_show_description: bool,
    /// Active pane when the ultra-wide Playing layout exposes its queue.
    pub playing_pane: PlayingPane,

    // Home dashboard presentation
    /// Which Home section holds the selection.
    pub home_section: HomeSection,
    /// Number of deduplicated recent tracks shown on Home, set during render.
    pub home_recent_len: usize,
    /// History entry count mirrored from the service before each render so
    /// selection movement works without giving state access to the service.
    pub history_len: usize,

    /// Displayed level-meter band heights, smoothed between level updates.
    pub(crate) viz_bands: [f32; crate::render::components::BAND_COUNT],

    /// Resolved icon mode from configuration (PRD 10.12).
    pub icon_mode: ratatube_domain::config::IconMode,
    /// Active color theme, mirrored from configuration and switched live by
    /// the ctrl+p settings menu.
    pub theme: ratatube_domain::config::ThemeName,
}
