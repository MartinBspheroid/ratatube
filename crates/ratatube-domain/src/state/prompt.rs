//! Prompt intent shared by the playlist commands and the client's prompt UI.

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
