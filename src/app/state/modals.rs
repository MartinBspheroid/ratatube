//! Prompt, picker, editor, and confirmation modal state.

/// State of the add-to-playlist picker modal.
#[derive(Debug, Clone)]
pub struct PickerState {
    /// Track to add on submit.
    pub track: crate::media::Track,
    /// Typed filter over playlist names.
    pub filter: String,
    /// Selection within the visible candidate list.
    pub selected: usize,
}

/// Purpose of a text prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptPurpose {
    SaveQueueAsPlaylist,
    RenamePlaylist,
    ImportPlaylistUrl,
    ImportPlaylistJson,
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

/// A yes/no confirmation dialog.
#[derive(Debug, Clone)]
pub struct ConfirmState {
    pub message: String,
    pub action: Box<crate::app::action::Action>,
}
