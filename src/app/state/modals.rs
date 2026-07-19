//! Prompt, picker, editor, and confirmation modal state.

use super::AppState;

/// The single topmost overlay that owns terminal input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModalCapture {
    TrackContext,
    TrackDetails,
    NotificationLog,
    SearchDetails,
    Import,
    Confirm,
    PlaylistEditor,
    Prompt,
    Picker,
}

impl AppState {
    /// Return the topmost modal that captures keyboard, mouse, and paste input.
    pub(crate) fn modal_capture(&self) -> Option<ModalCapture> {
        if self.track_context_menu.is_some() {
            Some(ModalCapture::TrackContext)
        } else if self.track_details_modal.is_some() {
            Some(ModalCapture::TrackDetails)
        } else if self.show_notification_log {
            Some(ModalCapture::NotificationLog)
        } else if self.search_detail_open {
            Some(ModalCapture::SearchDetails)
        } else if self.import.is_some() {
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
    pub(crate) fn show_playlist_picker(&mut self, track: crate::media::Track) {
        self.track_context_menu = None;
        self.picker = Some(PickerState {
            track,
            filter: String::new(),
            selected: 0,
        });
    }

    /// Replace any context menu with stable details for `track`.
    pub(crate) fn show_track_details(&mut self, track: crate::media::Track) {
        let details = if self
            .current_track
            .as_ref()
            .is_some_and(|current| current.id == track.id)
        {
            self.current_details.clone()
        } else {
            None
        };
        self.track_context_menu = None;
        self.track_details_modal = Some(TrackDetailsModalState { track, details });
    }
}

/// State of the universal track context menu.
#[derive(Debug, Clone)]
pub struct TrackContextMenuState {
    /// Resolved track, exact source occurrence, and ordered actions.
    pub context: crate::app::track_context::TrackContext,
    /// Selected action index.
    pub selected: usize,
}

/// Selected-track details independent from the current playback track.
#[derive(Debug, Clone)]
pub struct TrackDetailsModalState {
    /// Stable track captured by the context menu.
    pub track: crate::media::Track,
    /// Existing extended details only when they belong to this exact track.
    pub details: Option<crate::media::TrackDetails>,
}

/// State of the add-to-playlist picker modal.
#[derive(Debug, Clone)]
pub struct PickerState {
    /// Track to add on submit.
    pub track: crate::media::Track,
    /// Typed filter over playlist names; a non-matching name becomes a
    /// "create new playlist" entry.
    pub filter: String,
    /// Selection within the visible candidate list; index 0 may be "create new".
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

/// A yes/no confirmation dialog, such as playlist deletion (PRD 10.7).
#[derive(Debug, Clone)]
pub struct ConfirmState {
    pub message: String,
    pub action: Box<crate::app::action::Action>,
}
