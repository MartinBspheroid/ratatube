//! Application construction, service startup, and graceful shutdown.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::app::{App, PlaybackRecoveryResult, StartupIntent};
use crate::config::Config;
use crate::error::Result;
use crate::history::HistoryService;
use crate::persistence::AppPaths;
use crate::playback::{MpvProcess, PlaybackController, PlaybackEvent};
use crate::playlists::service::PlaylistService;

impl App {
    /// Build the app from loaded config and restored state.
    pub fn new(
        config: Config,
        paths: AppPaths,
        mut state: crate::app::state::AppState,
        picker: ratatui_image::picker::Picker,
    ) -> Self {
        state.icon_mode = crate::ui::icons::resolve_icon_mode(config.ui.icons);
        let yt_dlp = crate::media::yt_dlp::YtDlp::new(config.paths.yt_dlp.clone());
        let playlists = PlaylistService::new(paths.playlists_dir());
        let history = if config.history.enabled {
            HistoryService::load(&paths.history_file(), config.history.max_entries)
                .map_err(|err| tracing::warn!(?err, "history load failed"))
                .ok()
        } else {
            None
        };
        Self {
            state,
            config,
            paths,
            yt_dlp,
            mpv: None,
            playback: None,
            playlists,
            history,
            listening: crate::history::model::ListeningAccumulator::default(),
            last_click: None,
            picker,
            last_session_save: None,
            startup_intent: None,
            autoplay_first_search: false,
            prefetched: None,
            radio_fetching: false,
            operations: crate::app::operations::OperationRegistry::default(),
            playback_recovering: false,
            persistence_writer: None,
            filter_sync_key: None,
            list_revision: 0,
        }
    }

    /// Set the CLI-provided startup behavior; call before `run`.
    pub fn set_startup_intent(&mut self, intent: Option<StartupIntent>) {
        self.startup_intent = intent;
    }

    /// Load initial data (playlists) into state. Call before `run`.
    pub fn load_initial_data(&mut self) {
        match self.playlists.list() {
            Ok(playlists) => self.state.playlists = playlists,
            Err(err) => {
                tracing::warn!(?err, "playlist listing failed");
                self.state.notify("Could not load playlists", true);
            }
        }
    }

    /// Spawn persistent mpv and connect the IPC controller.
    pub async fn start_playback(&mut self, event_tx: mpsc::Sender<PlaybackEvent>) -> Result<()> {
        let (process, controller) = Self::connect_playback(
            self.config.paths.mpv.clone(),
            self.paths.data_dir.join("mpv.sock"),
            self.config.playback.default_volume,
            event_tx,
        )
        .await?;
        self.mpv = Some(process);
        self.playback = Some(controller);
        self.state.mpv_ready = true;
        Ok(())
    }

    /// Spawn mpv, await its socket, and return an initialized controller.
    pub(super) async fn connect_playback(
        binary: String,
        socket: std::path::PathBuf,
        volume: u8,
        event_tx: mpsc::Sender<PlaybackEvent>,
    ) -> PlaybackRecoveryResult {
        let mut process = MpvProcess::spawn(&binary, &socket, volume)?;
        process.wait_for_socket(Duration::from_secs(5)).await?;
        let ipc = crate::playback::ipc::MpvIpc::connect(&socket, event_tx).await?;
        let mut controller = PlaybackController::new(ipc);
        controller.observe_defaults().await?;
        Ok((process, controller))
    }

    /// Graceful shutdown: persist state and stop mpv (PRD section 14).
    pub async fn shutdown(&mut self) {
        self.capture_resume_point();
        self.maybe_save_session(self.state.playback.position_seconds, true);
        self.record_current(crate::history::model::PlaybackOutcome::Stopped);
        self.persist_queue();
        self.persist_history();
        if let Some(writer) = self.persistence_writer.take()
            && let Err(err) = writer.shutdown().await
        {
            eprintln!("warning: persistence flush failed during shutdown: {err}");
        }
        if let Some(mut process) = self.mpv.take() {
            process.shutdown().await;
        }
    }
}
