//! Application orchestration facade and shared runtime ownership.

pub mod action;
pub mod actions;
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
    listening: ListeningAccumulator,
    last_click: Option<(Instant, View, usize)>,
    picker: ratatui_image::picker::Picker,
    last_session_save: Option<Instant>,
    startup_intent: Option<StartupIntent>,
    autoplay_first_search: bool,
    prefetched: Option<(String, String, Instant)>,
    radio_fetching: bool,
    operations: OperationRegistry,
    playback_recovering: bool,
    persistence_writer: Option<crate::persistence::writer::PersistenceWriter>,
    filter_sync_key: Option<FilterSyncKey>,
    list_revision: u64,
}
