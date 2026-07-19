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
    /// Delete the playlist after explicit confirmation (PRD 10.7).
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
    /// Insert a bracketed-paste payload without treating embedded newlines as submit.
    PromptPaste(String),
    PromptBackspace,
    PromptSubmit,
    PromptCancel,
    /// Open the metadata editor for the active playlist.
    OpenPlaylistEditor,
    PlaylistEditorInput(char),
    PlaylistEditorBackspace,
    PlaylistEditorNextField,
    PlaylistEditorSubmit,
    PlaylistEditorCancel,
    ConfirmYes,
    ConfirmNo,
    OpenPlaylistPicker,
    /// Open the picker for a stable context-menu track.
    OpenPlaylistPickerForTrack(Track),
    PickerInput(char),
    PickerBackspace,
    PickerNext,
    PickerPrevious,
    /// Add the selected track to the chosen playlist, creating one when the
    /// picker points at its "new" entry, then close the picker.
    PickerSubmit,
    PickerCancel,
    RemoveSelectedFromPlaylist,
    /// Remove one captured playlist occurrence if its track still matches.
    RemoveTrackOccurrence {
        playlist_id: String,
        track_index: usize,
        expected_track: Track,
    },
    /// Move the selected playlist track up (-1) or down (+1).
    MoveSelectedInPlaylist(i32),
}
