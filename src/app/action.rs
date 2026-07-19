//! Application action facade for the event-driven architecture in PRD section 13.

pub use crate::app::actions::{
    HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction,
};

/// A state-changing intent, grouped by the responsibility that handles it.
#[derive(Debug, Clone)]
pub enum Action {
    /// Navigation, search, and selection intent.
    Navigation(NavigationAction),
    /// Playback and media-lifecycle intent.
    Playback(PlaybackAction),
    /// Queue mutation or queue-resolution intent.
    Queue(QueueAction),
    /// Playlist, import, and playlist-modal intent.
    Playlists(PlaylistAction),
    /// History, activity, or notification intent.
    History(HistoryAction),
}
