//! Application orchestration facade and shared runtime ownership.

pub mod action;
mod actions;
pub mod filter;
pub mod operations;
pub mod reducer;
pub mod state;

mod action_dispatch;
mod background;
mod browser;
mod effects;
mod input;
mod lifecycle;
mod media_tasks;
mod mouse;
mod persistence;
mod playback_followup;
mod playback_recovery;
mod playback_session;
mod playlist_picker;
mod playlists;
mod runtime;
mod selection;
mod service_actions;
mod startup;
mod thumbnails;

#[cfg(test)]
mod tests;

use std::time::Instant;

use crate::app::operations::OperationRegistry;
use crate::app::state::{AppState, HistoryViewMode, View};
use crate::config::Config;
use crate::error::Result;
use crate::history::HistoryService;
use crate::history::model::ListeningAccumulator;
use crate::media::yt_dlp::YtDlp;
use crate::persistence::AppPaths;
use crate::playback::{MpvProcess, PlaybackController};
use crate::playlists::service::PlaylistService;

type PlaybackRecoveryResult = Result<(MpvProcess, PlaybackController)>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilterSyncKey {
    view: View,
    history_mode: HistoryViewMode,
    filter: String,
    list_revision: u64,
}

/// What the CLI asked the app to do right after startup.
#[derive(Debug, Clone)]
pub enum StartupIntent {
    /// Restore the previous session and start playing immediately.
    Resume,
    /// Search for a query and play the first result.
    PlayQuery(String),
}

/// The running application. Owns state, services, and the event loop.
pub struct App {
    state: AppState,
    config: Config,
    paths: AppPaths,
    yt_dlp: YtDlp,
    mpv: Option<MpvProcess>,
    playback: Option<PlaybackController>,
    playlists: PlaylistService,
    history: Option<HistoryService>,
    /// Media-position based listened-duration accumulator.
    listening: ListeningAccumulator,
    /// Last selectable click; double-clicks require the same target.
    last_click: Option<(Instant, View, usize)>,
    /// Terminal graphics picker (Kitty on Ghostty, halfblocks fallback).
    picker: ratatui_image::picker::Picker,
    /// Throttle for session snapshot writes during playback.
    last_session_save: Option<Instant>,
    /// CLI-provided startup behavior (`--resume` / `play <query>`).
    startup_intent: Option<StartupIntent>,
    /// Auto-play the first result of the next completed search.
    autoplay_first_search: bool,
    /// Pre-resolved stream URL for the next queue track: track ID, URL, and
    /// resolution time. This avoids dead air between tracks.
    prefetched: Option<(String, String, Instant)>,
    /// Whether a radio refill request is in flight.
    radio_fetching: bool,
    /// Owns cancellable asynchronous work and rejects stale completions.
    operations: OperationRegistry,
    /// Whether a bounded mpv restart/reconnect attempt is active.
    playback_recovering: bool,
    /// Single ordered owner for queue, history, and session persistence.
    persistence_writer: Option<crate::persistence::writer::PersistenceWriter>,
    /// Last filter derivation, used to avoid rebuilding unchanged rows on
    /// playback and timer events.
    filter_sync_key: Option<FilterSyncKey>,
    /// Incremented whenever a filterable collection changes.
    list_revision: u64,
}
