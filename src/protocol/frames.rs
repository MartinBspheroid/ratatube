//! Frame and payload types crossing the daemon socket.

use serde::{Deserialize, Serialize};

use crate::app::state::DomainState;
use crate::media::{Track, TrackDetails};
use crate::playback::PlaybackSnapshot;
use crate::playlists::Playlist;
use crate::queue::Queue;

/// Frames a client sends to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientFrame {
    /// Handshake; must be the first frame on a connection.
    Hello { protocol: u32 },
    /// A correlated command; the daemon answers with a `Reply` of equal `id`.
    Command { id: u64, command: Box<Command> },
}

/// Frames the daemon sends to a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonFrame {
    /// Handshake answer carrying the full domain snapshot.
    Welcome {
        protocol: u32,
        snapshot: Box<Snapshot>,
    },
    /// Answer to the command with the same `id`.
    Reply {
        id: u64,
        #[serde(flatten)]
        result: ReplyResult,
    },
    /// Broadcast domain change.
    Event { event: Box<WireEvent> },
}

/// Success-or-error body of a reply, flattened into the frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplyResult {
    Result(ReplyBody),
    Error(String),
}

/// Commands a client may issue. Kept deliberately coarse; completion-style
/// actions never cross the wire (they are daemon-internal).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Command {
    PlayQuery {
        query: String,
    },
    PlayTrack {
        track: Track,
    },
    PlayPause,
    Stop,
    Next,
    Previous,
    Seek {
        seconds: i64,
    },
    Volume {
        delta: i8,
    },
    ToggleShuffle,
    CycleRepeat,
    QueueAdd {
        track: Track,
        next: bool,
    },
    QueueRemove {
        order_index: usize,
        expected_revision: u64,
    },
    QueueMove {
        from: usize,
        to: usize,
    },
    QueueClear,
    QueueUndo,
    Search {
        query: String,
    },
    /// Absolute seek to a position in seconds (chapter jumps).
    SeekAbsolute {
        seconds: f64,
    },
    /// Replace or extend the queue with a stored playlist's tracks.
    PlaylistLoad {
        id: String,
        append: bool,
    },
    /// Delete a stored playlist (the client confirms before sending).
    PlaylistDelete {
        id: String,
    },
    /// Create a new empty playlist.
    PlaylistCreate {
        name: String,
    },
    /// Save the current queue as a playlist.
    SaveQueueAsPlaylist {
        name: String,
    },
    /// Remove one exact track occurrence from a stored playlist.
    PlaylistRemoveTrack {
        playlist_id: String,
        track_index: usize,
        expected_revision: u64,
    },
    /// Clear playback history (the client confirms before sending).
    HistoryClear,
    /// Delete one history entry, guarded by its track id.
    HistoryDelete {
        index: usize,
        expected_track_id: String,
    },
    /// Clear the Home activity log.
    ActivityClear,
    /// Add a track to a stored playlist by id.
    PlaylistAddTrack {
        playlist_id: String,
        track: Track,
    },
    /// Create a playlist named after the picker filter and add the track.
    PlaylistAddTrackNew {
        name: String,
        track: Track,
    },
    /// Rename a stored playlist by id.
    PlaylistRename {
        id: String,
        name: String,
    },
    /// Replace a stored playlist's name and description by id.
    PlaylistEdit {
        id: String,
        name: String,
        description: String,
    },
    /// Reorder a stored playlist's tracks by id.
    PlaylistMoveTrack {
        id: String,
        from: usize,
        to: usize,
    },
    ToggleMute,
    SpeedUp,
    SpeedDown,
    SpeedReset,
    CycleSleepTimer,
    ToggleRadio,
    /// Jump the queue cursor to an order position and play it.
    PlayQueuePosition {
        position: usize,
    },
    /// Resume a previous-session track at a stored position.
    Resume {
        track: Track,
        position_seconds: f64,
    },
    /// Fetch one exact video by URL (client classified the input).
    SearchExact {
        url: String,
    },
    /// Start a supervised remote playlist import.
    ImportStart {
        url: String,
    },
    /// Persist the reviewed import.
    ImportConfirm,
    /// Abandon the import flow.
    ImportCancel,
    /// Parse and save pasted playlist JSON (daemon re-validates).
    ImportJson {
        json: String,
    },
    Status,
    Shutdown,
}

/// Reply payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "body", rename_all = "snake_case")]
pub enum ReplyBody {
    Ack,
    Status { snapshot: Box<Snapshot> },
    Tracks { tracks: Vec<Track> },
}

/// External-tool availability mirrored to clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub mpv_ready: bool,
    pub yt_dlp_ready: bool,
}

/// Full mirrorable domain state sent on connect and on `Status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub queue: Queue,
    pub queue_revision: u64,
    pub playback: PlaybackSnapshot,
    pub current_track: Option<Track>,
    pub current_details: Option<TrackDetails>,
    pub playlists: Vec<Playlist>,
    pub playlists_revision: u64,
    pub health: Health,
}

impl From<&DomainState> for Snapshot {
    fn from(domain: &DomainState) -> Self {
        Self {
            queue: domain.queue.clone(),
            queue_revision: domain.queue_revision,
            playback: domain.playback.clone(),
            current_track: domain.current_track.clone(),
            current_details: domain.current_details.clone(),
            playlists: domain.playlists.clone(),
            playlists_revision: domain.playlists_revision,
            health: Health {
                mpv_ready: domain.mpv_ready,
                yt_dlp_ready: domain.yt_dlp_ready,
            },
        }
    }
}

/// Broadcast domain changes: `DomainEvent` kinds carrying fresh payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireEvent {
    QueueChanged {
        queue: Queue,
        queue_revision: u64,
    },
    PlaybackProgress {
        playback: PlaybackSnapshot,
    },
    TrackChanged {
        track: Option<Track>,
    },
    TrackDetailsChanged {
        details: Option<TrackDetails>,
    },
    PlaylistsChanged {
        playlists: Vec<Playlist>,
        playlists_revision: u64,
    },
    HistoryChanged,
    ImportChanged {
        import: Option<WireImport>,
    },
    Health {
        health: Health,
    },
}

/// Import-flow state mirrored to clients (operation identity stays
/// daemon-internal).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum WireImport {
    Fetching {
        url: String,
    },
    Review {
        summary: crate::playlists::import::ImportSummary,
        playlist: Box<Playlist>,
    },
    Failed {
        url: String,
        message: String,
    },
}

impl WireImport {
    /// Mirror the daemon's import state onto the wire.
    pub fn from_state(state: &crate::app::state::ImportState) -> Self {
        use crate::app::state::ImportState;
        match state {
            ImportState::Fetching { url, .. } => Self::Fetching { url: url.clone() },
            ImportState::Review { summary, playlist } => Self::Review {
                summary: summary.clone(),
                playlist: playlist.clone(),
            },
            ImportState::Failed { url, message } => Self::Failed {
                url: url.clone(),
                message: message.clone(),
            },
        }
    }

    /// Reconstruct client-side import state (placeholder operation id).
    pub fn into_state(self) -> crate::app::state::ImportState {
        use crate::app::state::ImportState;
        match self {
            Self::Fetching { url } => ImportState::Fetching {
                operation_id: crate::app::operations::OperationId::mirror_placeholder(),
                url,
            },
            Self::Review { summary, playlist } => ImportState::Review { summary, playlist },
            Self::Failed { url, message } => ImportState::Failed { url, message },
        }
    }
}
