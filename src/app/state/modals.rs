//! Prompt, picker, editor, and confirmation modal state.

/// State of the universal track context menu.
#[derive(Debug, Clone)]
pub struct TrackContextMenuState {
    /// Resolved track, exact source occurrence, and ordered actions.
    pub context: crate::app::track_context::TrackContext,
    /// Selected action index.
    pub selected: usize,
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
