//! Playlist, import, and playlist-modal actions.

use crate::app::operations::OperationId;
use crate::app::state::PromptPurpose;
use crate::media::Track;
use crate::media::yt_dlp::ImportRejections;
use crate::playlists::Playlist;

/// An intent that manages playlists, imports, or their modal workflows.
#[derive(Debug, Clone)]
pub enum PlaylistAction {
    DeleteSelectedPlaylist,
    OpenPlaylistDetail,
    SaveQueueAsPlaylist(String),
    CreatePlaylist(String),
    RenameSelectedPlaylist(String),
    LoadPlaylistIntoQueue(String),
    AppendPlaylistToQueue(String),
    DeletePlaylist(String),
    DeletePlaylistConfirmed(String),
    PlaylistSaved(Playlist),
    StartImport(String),
    ImportStarted {
        operation_id: OperationId,
        url: String,
    },
    ImportCompleted {
        operation_id: OperationId,
        url: String,
        title: String,
        remote_id: Option<String>,
        tracks: Vec<Track>,
        rejections: ImportRejections,
    },
    ImportFailed {
        operation_id: OperationId,
        url: String,
        message: String,
    },
    ConfirmImport,
    CancelImport,
    OpenPrompt(PromptPurpose),
    PromptInput(char),
    /// Insert bracketed-paste content without treating newlines as submit.
    PromptPaste(String),
    PromptBackspace,
    PromptSubmit,
    PromptCancel,
    OpenPlaylistEditor,
    PlaylistEditorInput(char),
    PlaylistEditorBackspace,
    PlaylistEditorNextField,
    PlaylistEditorSubmit,
    PlaylistEditorCancel,
    ConfirmYes,
    ConfirmNo,
    OpenPlaylistPicker,
    PickerInput(char),
    PickerBackspace,
    PickerNext,
    PickerPrevious,
    PickerSubmit,
    PickerCancel,
    RemoveSelectedFromPlaylist,
    /// Move the selected playlist track up or down.
    MoveSelectedInPlaylist(i32),
}
