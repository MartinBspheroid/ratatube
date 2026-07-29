//! Playlist, import, and playlist-modal actions.

use crate::media::Track;
use crate::media::yt_dlp::ImportRejections;
use crate::operations::OperationId;
use crate::playlists::Playlist;
use crate::state::PromptPurpose;

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
        expected_revision: u64,
    },
    /// Add a track to a stored playlist by id (daemon clients + picker).
    AddTrackToPlaylist {
        playlist_id: String,
        track: Track,
    },
    /// Create a playlist and add the track to it (picker "create new").
    AddTrackToNewPlaylist {
        name: String,
        track: Track,
    },
    /// Rename a stored playlist by id (daemon clients).
    RenamePlaylist {
        id: String,
        name: String,
    },
    /// Replace a stored playlist's name and description by id.
    EditPlaylist {
        id: String,
        name: String,
        description: String,
    },
    /// Reorder a stored playlist's tracks by id (daemon clients).
    MoveTrackInPlaylist {
        id: String,
        from: usize,
        to: usize,
    },
    /// Parse and save pasted playlist JSON (daemon clients).
    ImportPlaylistsJson(String),
    /// Move the selected playlist track up (-1) or down (+1).
    MoveSelectedInPlaylist(i32),
}
