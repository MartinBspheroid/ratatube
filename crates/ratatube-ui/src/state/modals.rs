//! Prompt, picker, editor, and confirmation modal state.

use ratatube_domain::state::PromptPurpose;

use super::AppState;

/// The single topmost overlay that owns terminal input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalCapture {
    TrackContext,
    TrackDetails,
    NotificationLog,
    Settings,
    SearchDetails,
    Import,
    Confirm,
    PlaylistEditor,
    Prompt,
    Picker,
}

impl crate::state::UiState {
    /// Return the topmost modal that captures keyboard, mouse, and paste
    /// input. `import_active` is the domain-side import flow, whose review
    /// modal also captures input.
    pub fn modal_capture(&self, import_active: bool) -> Option<ModalCapture> {
        if self.track_context_menu.is_some() {
            Some(ModalCapture::TrackContext)
        } else if self.track_details_modal.is_some() {
            Some(ModalCapture::TrackDetails)
        } else if self.show_notification_log {
            Some(ModalCapture::NotificationLog)
        } else if self.settings.is_some() {
            // Above the import overlay so another client's import cannot
            // steal input from a locally opened settings menu.
            Some(ModalCapture::Settings)
        } else if self.search_detail_open {
            Some(ModalCapture::SearchDetails)
        } else if import_active {
            Some(ModalCapture::Import)
        } else if self.confirm.is_some() {
            Some(ModalCapture::Confirm)
        } else if self.playlist_editor.is_some() {
            Some(ModalCapture::PlaylistEditor)
        } else if self.prompt.is_some() {
            Some(ModalCapture::Prompt)
        } else if self.picker.is_some() {
            Some(ModalCapture::Picker)
        } else {
            None
        }
    }

    /// Replace any context menu with an add-to-playlist picker for `track`.
    pub fn show_playlist_picker(&mut self, track: ratatube_domain::media::Track) {
        self.track_context_menu = None;
        self.picker = Some(PickerState {
            track,
            filter: String::new(),
            selected: 0,
        });
    }

    /// Replace any context menu with stable details for `track`; `details`
    /// are supplied by the caller only when they belong to this exact track.
    pub fn show_track_details(
        &mut self,
        track: ratatube_domain::media::Track,
        details: Option<ratatube_domain::media::TrackDetails>,
    ) {
        self.track_context_menu = None;
        self.track_details_modal = Some(TrackDetailsModalState { track, details });
    }
}

impl AppState {
    /// Return the topmost modal that captures keyboard, mouse, and paste input.
    pub fn modal_capture(&self) -> Option<ModalCapture> {
        self.ui.modal_capture(self.domain.import.is_some())
    }

    /// Replace any context menu with an add-to-playlist picker for `track`.
    pub fn show_playlist_picker(&mut self, track: ratatube_domain::media::Track) {
        self.ui.show_playlist_picker(track);
    }

    /// Replace any context menu with stable details for `track`.
    pub fn show_track_details(&mut self, track: ratatube_domain::media::Track) {
        let details = self
            .domain
            .current_track
            .as_ref()
            .is_some_and(|current| current.id == track.id)
            .then(|| self.domain.current_details.clone())
            .flatten();
        self.ui.show_track_details(track, details);
    }
}

/// State of the universal track context menu.
#[derive(Debug, Clone)]
pub struct TrackContextMenuState {
    /// Resolved track, exact source occurrence, and ordered actions.
    pub context: crate::state::track_context::TrackContext,
    /// Selected action index.
    pub selected: usize,
}

/// Selected-track details independent from the current playback track.
#[derive(Debug, Clone)]
pub struct TrackDetailsModalState {
    /// Stable track captured by the context menu.
    pub track: ratatube_domain::media::Track,
    /// Existing extended details only when they belong to this exact track.
    pub details: Option<ratatube_domain::media::TrackDetails>,
}

/// State of the add-to-playlist picker modal.
#[derive(Debug, Clone)]
pub struct PickerState {
    /// Track to add on submit.
    pub track: ratatube_domain::media::Track,
    /// Typed filter over playlist names; a non-matching name becomes a
    /// "create new playlist" entry.
    pub filter: String,
    /// Selection within the visible candidate list; index 0 may be "create new".
    pub selected: usize,
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

/// A yes/no confirmation dialog, such as playlist deletion (PRD 10.7).
#[derive(Debug, Clone)]
pub struct ConfirmState {
    pub message: String,
    pub action: Box<ratatube_domain::action::Action>,
}

/// Tab within the ctrl+p settings menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    /// Theme selection with live preview.
    Appearance,
    /// Icon mode and resume-on-launch behavior.
    General,
}

/// Number of value rows on the General settings tab (icons, resume).
pub const SETTINGS_GENERAL_ROWS: usize = 2;

/// State of the ctrl+p settings menu. Theme selection previews live on the
/// running UI; Enter persists every draft, Esc restores the opening theme.
#[derive(Debug, Clone)]
pub struct SettingsState {
    pub tab: SettingsTab,
    /// Selected row within the active tab: a theme on Appearance, a value
    /// row on General.
    pub selected: usize,
    /// Theme active when the menu opened; restored on cancel.
    pub original_theme: ratatube_domain::config::ThemeName,
    /// Draft icon mode (the configured value, not the resolved mode).
    pub icons: ratatube_domain::config::IconMode,
    /// Draft resume-on-launch behavior.
    pub resume: ratatube_domain::config::ResumeMode,
}
