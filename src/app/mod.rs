//! Application orchestration: event loop wiring input, timers, mpv events,
//! and yt-dlp tasks into actions and effects (PRD sections 13 and 15).

pub mod action;
pub mod filter;
pub mod operations;
pub mod reducer;
pub mod state;

use std::time::{Duration, Instant};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use crate::app::action::Action;
use crate::app::operations::{OperationKind, OperationRegistry};
use crate::app::reducer::{Effect, reduce};
use crate::app::state::{AppState, Focus, ImportState, PromptPurpose, View};
use crate::config::Config;
use crate::error::Result;
use crate::history::{
    HistoryService, model::HistoryEntry, model::ListeningAccumulator, model::PlaybackOutcome,
};
use crate::input::keymap;
use crate::media::Track;
use crate::media::import::{InputKind, classify_input};
use crate::media::yt_dlp::YtDlp;
use crate::persistence::AppPaths;
use crate::playback::{MpvProcess, PlaybackController, PlaybackEvent};
use crate::playlists::Playlist;
use crate::playlists::service::PlaylistService;

type PlaybackRecoveryResult = Result<(MpvProcess, PlaybackController)>;

#[derive(Debug, Clone, Copy)]
enum ThumbnailPurpose {
    CurrentTrack,
    SearchSelection,
}

impl ThumbnailPurpose {
    const fn operation_kind(self) -> OperationKind {
        match self {
            Self::CurrentTrack => OperationKind::Thumbnail,
            Self::SearchSelection => OperationKind::SearchThumbnail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilterSyncKey {
    view: View,
    history_mode: crate::app::state::HistoryViewMode,
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
    /// CLI-provided startup behavior (--resume / play <query>).
    startup_intent: Option<StartupIntent>,
    /// Auto-play the first result of the next completed search.
    autoplay_first_search: bool,
    /// Pre-resolved stream URL for the next queue track: (track id, url,
    /// resolved at). Kills the dead air between tracks.
    prefetched: Option<(String, String, Instant)>,
    /// A radio refill request is in flight.
    radio_fetching: bool,
    /// Owns cancellable asynchronous work and rejects stale completions.
    operations: OperationRegistry,
    /// True while a bounded mpv restart/reconnect attempt is active.
    playback_recovering: bool,
    /// Single ordered owner for queue/history/session persistence.
    persistence_writer: Option<crate::persistence::writer::PersistenceWriter>,
    /// Avoid rebuilding unchanged filtered rows on playback/timer events.
    filter_sync_key: Option<FilterSyncKey>,
    /// Incremented whenever a filterable collection changes.
    list_revision: u64,
}

impl App {
    /// Build the app from loaded config and restored state.
    pub fn new(
        config: Config,
        paths: AppPaths,
        mut state: AppState,
        picker: ratatui_image::picker::Picker,
    ) -> Self {
        state.icon_mode = crate::ui::icons::resolve_icon_mode(config.ui.icons);
        let yt_dlp = YtDlp::new(config.paths.yt_dlp.clone());
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
            listening: ListeningAccumulator::default(),
            last_click: None,
            picker,
            last_session_save: None,
            startup_intent: None,
            autoplay_first_search: false,
            prefetched: None,
            radio_fetching: false,
            operations: OperationRegistry::default(),
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

    async fn connect_playback(
        binary: String,
        socket: std::path::PathBuf,
        volume: u8,
        event_tx: mpsc::Sender<PlaybackEvent>,
    ) -> Result<(MpvProcess, PlaybackController)> {
        let mut process = MpvProcess::spawn(&binary, &socket, volume)?;
        process.wait_for_socket(Duration::from_secs(5)).await?;
        let ipc = crate::playback::ipc::MpvIpc::connect(&socket, event_tx).await?;
        let mut controller = PlaybackController::new(ipc);
        controller.observe_defaults().await?;
        Ok((process, controller))
    }

    /// Main event loop: terminal input, mpv events, render tick.
    pub async fn run(&mut self, terminal: &mut crate::ui::Terminal) -> Result<()> {
        let (action_tx, mut action_rx) = mpsc::channel::<Action>(256);
        let (playback_tx, mut playback_rx) = mpsc::channel::<PlaybackEvent>(256);
        let (recovery_tx, mut recovery_rx) =
            mpsc::channel::<(crate::app::operations::OperationId, PlaybackRecoveryResult)>(1);
        let (persistence_writer, mut persistence_rx) =
            crate::persistence::writer::PersistenceWriter::new();
        self.persistence_writer = Some(persistence_writer);

        if self.config.paths.mpv != "false"
            && let Err(err) = self.start_playback(playback_tx.clone()).await
        {
            self.state.notify(&format!("mpv unavailable: {err}"), true);
        }

        self.init_session(&action_tx).await;

        let mut events = EventStream::new();
        let mut tick = tokio::time::interval(Duration::from_millis(
            self.config.ui.progress_refresh_ms.max(100),
        ));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        while self.state.running {
            tokio::select! {
                maybe_event = events.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            self.handle_key(key, &action_tx).await;
                        }
                        Some(Ok(Event::Mouse(mouse))) => {
                            self.handle_mouse(mouse, &action_tx).await;
                        }
                        Some(Ok(Event::Paste(text))) if self.state.prompt.is_some() => {
                            let _ = action_tx.send(Action::PromptPaste(text)).await;
                        }
                        _ => {}
                    }
                }
                Some(event) = playback_rx.recv() => {
                    tracing::debug!(event_type = ?std::mem::discriminant(&event), "mpv event");
                    if let Some(playback) = self.playback.as_mut() {
                        playback.on_event(&event);
                    }
                    self.on_playback_event(&event);
                    if event == PlaybackEvent::Shutdown && self.state.running {
                        self.begin_playback_recovery(
                            playback_tx.clone(),
                            recovery_tx.clone(),
                        );
                    }
                    let _ = action_tx.send(Action::PlaybackEvent(event)).await;
                }
                Some((operation_id, result)) = recovery_rx.recv() => {
                    if self
                        .operations
                        .complete(OperationKind::PlaybackRecovery, operation_id)
                    {
                        self.playback_recovering = false;
                        match result {
                            Ok((process, controller)) => {
                                self.mpv = Some(process);
                                self.playback = Some(controller);
                                self.state.mpv_ready = true;
                                self.state.notify("mpv reconnected", false);
                            }
                            Err(err) => {
                                self.state.notify(&format!("mpv restart failed: {err}"), true);
                            }
                        }
                    }
                }
                Some(outcome) = persistence_rx.recv() => {
                    match outcome.result {
                        Ok(()) => tracing::debug!(key = %outcome.key, "persistence completed"),
                        Err(err) => {
                            tracing::warn!(?err, key = %outcome.key, "persistence failed");
                            self.state.notify(
                                &format!(
                                    "Could not save {}: {err}. Changes are not durable.",
                                    outcome.description
                                ),
                                true,
                            );
                        }
                    }
                }
                Some(action) = action_rx.recv() => {
                    self.handle_action(action, &action_tx).await;
                }
                _ = tick.tick() => {
                    self.state.tick_spinner();
                    if self.state.notification.as_ref().is_some_and(|notification| {
                        notification.is_expired_at(Instant::now())
                    }) {
                        self.state.notification = None;
                    }
                    // Sleep timer: stop playback once the deadline passes.
                    if let Some(timer) = self.state.sleep_timer
                        && timer.deadline <= Instant::now()
                    {
                        self.state.sleep_timer = None;
                        self.state.notify("Sleep timer: stopping playback", false);
                        let _ = action_tx.send(Action::Stop).await;
                    }
                }
            }
            // Refresh filtered indices and the mirrored history length so
            // selection movement matches what is rendered.
            self.sync_list_view();
            crate::ui::render_with(terminal, &mut self.state, self.history.as_ref())?;
        }
        self.operations.shutdown(Duration::from_secs(1)).await;
        Ok(())
    }

    fn begin_playback_recovery(
        &mut self,
        event_tx: mpsc::Sender<PlaybackEvent>,
        recovery_tx: mpsc::Sender<(crate::app::operations::OperationId, PlaybackRecoveryResult)>,
    ) {
        if self.playback_recovering || self.config.paths.mpv == "false" {
            return;
        }
        self.playback_recovering = true;
        self.playback = None;
        self.mpv = None;
        let ticket = self.operations.start(OperationKind::PlaybackRecovery);
        let operation_id = ticket.id();
        let cancellation = ticket.cancellation().clone();
        let binary = self.config.paths.mpv.clone();
        let socket = self.paths.data_dir.join("mpv.sock");
        let volume = self.config.playback.default_volume;
        let handle = tokio::spawn(async move {
            let recover = async {
                let mut last_error = None;
                for attempt in 0..3 {
                    match Self::connect_playback(
                        binary.clone(),
                        socket.clone(),
                        volume,
                        event_tx.clone(),
                    )
                    .await
                    {
                        Ok(components) => return Ok(components),
                        Err(err) => last_error = Some(err),
                    }
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                }
                Err(last_error.expect("at least one recovery attempt"))
            };
            let result = tokio::select! {
                () = cancellation.cancelled() => return,
                result = recover => result,
            };
            let _ = recovery_tx.send((operation_id, result)).await;
        });
        self.operations
            .attach(OperationKind::PlaybackRecovery, operation_id, handle);
    }

    /// Restore the previous session (armed resume) and apply any CLI
    /// startup intent. Called once, right after mpv startup.
    async fn init_session(&mut self, action_tx: &mpsc::Sender<Action>) {
        use crate::config::ResumeMode;

        let mode = match self.startup_intent {
            Some(StartupIntent::Resume) => ResumeMode::Playing,
            _ => self.config.playback.resume_on_launch,
        };
        let session = crate::persistence::session::load(&self.paths.session_file());
        if let Some(document) = &session {
            self.state.activity = document.activity.clone();
            self.state.resume_points = document.resume_points.clone();
        }

        if let Some(StartupIntent::PlayQuery(query)) = self.startup_intent.clone() {
            self.state.view = View::Search;
            self.autoplay_first_search = true;
            self.submit_text_query(query, action_tx).await;
            return;
        }

        if mode == ResumeMode::Off || self.playback.is_none() {
            return;
        }
        let Some(doc) = session else {
            return;
        };
        let Some(track) = doc.track else {
            return;
        };

        if let Some(p) = self.playback.as_mut()
            && doc.volume > 0
            && doc.volume != self.config.playback.default_volume
        {
            let _ = p.set_volume(doc.volume).await;
        }

        // Align the restored queue's cursor with the session track so
        // next/previous continue from the right place.
        if let Some(pos) = self
            .state
            .queue
            .order
            .iter()
            .position(|&i| self.state.queue.tracks[i].id == track.id)
        {
            self.state.queue.position = Some(pos);
        }

        self.state.current_track = Some(track.clone());
        self.state.playback.position_seconds = doc.position_seconds;
        self.state.playback.duration_seconds = track.duration_seconds.map(|d| d as f64);
        self.state.pending_resume = Some(crate::app::state::PendingResume {
            track: track.clone(),
            position_seconds: doc.position_seconds,
            armed: false,
            play_on_load: mode == ResumeMode::Playing,
        });

        self.spawn_pending_resume_resolution(track, action_tx);
    }

    /// Persist the session snapshot, throttled to one write per interval.
    fn maybe_save_session(&mut self, position_seconds: f64, force: bool) {
        const SESSION_SAVE_INTERVAL: Duration = Duration::from_secs(5);
        let due = self
            .last_session_save
            .is_none_or(|t| t.elapsed() >= SESSION_SAVE_INTERVAL);
        if !force && !due {
            return;
        }
        self.last_session_save = Some(Instant::now());
        let mut doc = crate::persistence::session::SessionDocument::new(
            self.state.current_track.clone(),
            position_seconds,
            self.state.playback.volume,
        );
        doc.activity = self.state.activity.clone();
        doc.resume_points = self.state.resume_points.clone();
        let path = self.paths.session_file();
        self.submit_persistence("session", "session", move || {
            crate::persistence::session::save(&path, &doc)
        });
    }

    /// Mouse input: wheel scrolls the list, click selects, double-click plays.
    async fn handle_mouse(
        &mut self,
        mouse: crossterm::event::MouseEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        use crossterm::event::{MouseButton, MouseEventKind};
        if self.state.prompt.is_some()
            || self.state.confirm.is_some()
            || self.state.import.is_some()
            || self.state.picker.is_some()
            || self.state.search_detail_open
        {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if self.state.view == View::NowPlaying {
                    let _ = action_tx.send(Action::ScrollNowPlaying(3)).await;
                } else {
                    for _ in 0..3 {
                        let _ = action_tx.send(Action::SelectNext).await;
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.state.view == View::NowPlaying {
                    let _ = action_tx.send(Action::ScrollNowPlaying(-3)).await;
                } else {
                    for _ in 0..3 {
                        let _ = action_tx.send(Action::SelectPrevious).await;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Header row: click a tab to switch views.
                if mouse.row == 0 {
                    let icons = crate::ui::icons::icons_for(self.state.icon_mode);
                    let narrow =
                        crate::ui::layout::Breakpoint::from_width(self.state.screen_area.width)
                            == crate::ui::layout::Breakpoint::Narrow;
                    for (view, start, end) in
                        crate::ui::widgets::tab_hit_zones(&icons, self.state.view, narrow)
                    {
                        if mouse.column >= start && mouse.column < end {
                            let _ = action_tx.send(Action::Navigate(view)).await;
                            return;
                        }
                    }
                    return;
                }
                // Now-playing panel: click the progress row to seek.
                if self.state.has_now_playing() {
                    let layout =
                        crate::ui::layout::AppLayout::new(self.state.screen_area, true, true);
                    let bar = layout.now_playing;
                    // The five-row bar has three bordered content rows; the
                    // progress gauge is the middle content row.
                    let gauge_row = bar.y + 2;
                    if bar.height >= 5
                        && mouse.row == gauge_row
                        && mouse.column > bar.x
                        && mouse.column < bar.x + bar.width.saturating_sub(1)
                    {
                        let inner_width = bar.width.saturating_sub(2);
                        let fraction =
                            f64::from(mouse.column - bar.x - 1) / f64::from(inner_width.max(1));
                        let _ = action_tx.send(Action::SeekToFraction(fraction)).await;
                        return;
                    }
                }
                let area = self.state.list_hit_area;
                if mouse.column >= area.x
                    && mouse.column < area.x + area.width
                    && mouse.row >= area.y
                    && mouse.row < area.y + area.height
                {
                    let offset = if self.state.view == View::Search {
                        self.state.table_state.offset()
                    } else {
                        self.state.list_state.offset()
                    };
                    let index = offset + usize::from(mouse.row - area.y);
                    if index < self.state.active_list_len() {
                        self.state.selected_index = index;
                        let now = Instant::now();
                        let target = (self.state.view, index);
                        let double_click = self.last_click.is_some_and(|(at, view, clicked)| {
                            (view, clicked) == target
                                && now.duration_since(at) < Duration::from_millis(400)
                        });
                        self.last_click = Some((now, target.0, target.1));
                        if double_click {
                            let _ = action_tx.send(Action::PlaySelected).await;
                            self.last_click = None;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Reduce an action and execute resulting effects plus service work.
    async fn handle_action(&mut self, action: Action, action_tx: &mpsc::Sender<Action>) {
        if action_changes_filterable_data(&action) {
            self.list_revision = self.list_revision.wrapping_add(1);
            self.filter_sync_key = None;
        }
        match &action {
            Action::PlaybackResolved { operation_id, .. }
            | Action::PlaybackResolveFailed { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::Playback, *operation_id)
                {
                    return;
                }
            }
            Action::ImportCompleted { operation_id, .. }
            | Action::ImportFailed { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::Import, *operation_id)
                {
                    return;
                }
            }
            Action::CancelImport => self.operations.cancel(OperationKind::Import),
            Action::RadioTracksLoaded { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::Radio, *operation_id)
                {
                    return;
                }
            }
            Action::DetailsLoaded { operation_id, .. }
            | Action::DetailsFailed { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::Details, *operation_id)
                {
                    return;
                }
            }
            Action::PrefetchResolved { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::Prefetch, *operation_id)
                {
                    return;
                }
            }
            Action::ThumbnailLoaded { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::Thumbnail, *operation_id)
                {
                    return;
                }
            }
            Action::SearchThumbnailLoaded { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::SearchThumbnail, *operation_id)
                {
                    return;
                }
            }
            Action::SessionStreamResolved { operation_id, .. }
            | Action::SessionResolveFailed { operation_id, .. } => {
                if !self
                    .operations
                    .complete(OperationKind::Session, *operation_id)
                {
                    return;
                }
            }
            Action::MixLoaded { operation_id, .. } => {
                if !self.operations.complete(OperationKind::Mix, *operation_id) {
                    return;
                }
            }
            Action::ToggleRadio if self.state.radio => {
                self.operations.cancel(OperationKind::Radio);
                self.radio_fetching = false;
            }
            _ => {}
        }
        tracing::debug!(action_type = ?std::mem::discriminant(&action), "action");
        // Record skips before the reducer replaces the current track.
        match &action {
            Action::NextTrack | Action::PreviousTrack => {
                self.capture_resume_point();
                self.record_current(PlaybackOutcome::Skipped);
            }
            Action::Stop | Action::PlayTrack(_) | Action::PlaybackResolved { .. } => {
                self.capture_resume_point();
            }
            Action::ThumbnailLoaded {
                track_id, bytes, ..
            } => {
                self.on_thumbnail_loaded(track_id.clone(), bytes.clone());
            }
            Action::SearchThumbnailLoaded {
                track_id, bytes, ..
            } => {
                self.on_search_thumbnail_loaded(track_id.clone(), bytes.clone());
            }
            Action::SeekForward
            | Action::SeekBackward
            | Action::SeekForwardLarge
            | Action::SeekBackwardLarge
            | Action::SeekToFraction(_)
            | Action::NextChapter
            | Action::PreviousChapter => self.listening.seeking(),
            _ => {}
        }
        let effects = reduce(&mut self.state, action.clone());
        self.execute(effects, action_tx).await;
        self.handle_service_action(action, action_tx).await;
        self.sync_search_thumbnail(action_tx);
    }

    /// Route keyboard input: modals first, then focus-based keymap.
    async fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        // Notification log overlay: any dismiss key closes it.
        if self.state.show_notification_log {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('!') | KeyCode::Char('q') | KeyCode::Enter
            ) {
                self.state.show_notification_log = false;
            }
            return;
        }
        if self.state.search_detail_open && matches!(key.code, KeyCode::Esc | KeyCode::Char('i')) {
            let _ = action_tx.send(Action::ToggleSearchDetail).await;
            return;
        }
        // Import operation/review modal: Esc always cancels; Enter confirms
        // only a successfully fetched review.
        if let Some(import) = &self.state.import {
            let action = match (import, key.code) {
                (ImportState::Review { .. }, KeyCode::Enter) => Some(Action::ConfirmImport),
                (_, KeyCode::Esc) => Some(Action::CancelImport),
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        // Confirmation dialog: y/n.
        if self.state.confirm.is_some() {
            let action = match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => Some(Action::ConfirmYes),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(Action::ConfirmNo),
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        if self.state.playlist_editor.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::PlaylistEditorSubmit),
                KeyCode::Esc => Some(Action::PlaylistEditorCancel),
                KeyCode::Tab | KeyCode::BackTab => Some(Action::PlaylistEditorNextField),
                KeyCode::Backspace => Some(Action::PlaylistEditorBackspace),
                KeyCode::Char(character) => Some(Action::PlaylistEditorInput(character)),
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        // Text prompt modal.
        if self.state.prompt.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::PromptSubmit),
                KeyCode::Esc => Some(Action::PromptCancel),
                KeyCode::Backspace => Some(Action::PromptBackspace),
                KeyCode::Char(c) => Some(Action::PromptInput(c)),
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        // Add-to-playlist picker modal.
        if self.state.picker.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::PickerSubmit),
                KeyCode::Esc => Some(Action::PickerCancel),
                KeyCode::Backspace => Some(Action::PickerBackspace),
                KeyCode::Down => Some(Action::PickerNext),
                KeyCode::Up => Some(Action::PickerPrevious),
                KeyCode::Char(c) => Some(Action::PickerInput(c)),
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }

        match self.state.focus {
            Focus::SearchInput => {
                if keymap::submits_search(&key) {
                    let query = self.state.search_input.trim().to_string();
                    self.submit_text_query(query, action_tx).await;
                    return;
                }
                if keymap::leaves_search(&key) {
                    self.state.focus = Focus::Content;
                    return;
                }
                if let Some(action) = keymap::search_input_action(&key) {
                    let _ = action_tx.send(action).await;
                }
            }
            Focus::ListFilter => {
                match key.code {
                    // Enter locks the filter (list keys work on the matches);
                    // Esc clears it entirely.
                    KeyCode::Enter => {
                        if self
                            .state
                            .list_filter
                            .as_deref()
                            .is_none_or(|f| f.trim().is_empty())
                        {
                            self.state.list_filter = None;
                        }
                        self.state.focus = Focus::Content;
                    }
                    KeyCode::Esc => {
                        self.state.list_filter = None;
                        self.state.focus = Focus::Content;
                    }
                    KeyCode::Backspace => {
                        if let Some(filter) = &mut self.state.list_filter
                            && filter.pop().is_none()
                        {
                            self.state.list_filter = None;
                            self.state.focus = Focus::Content;
                        }
                        self.state.selected_index = 0;
                    }
                    KeyCode::Char(c) => {
                        if let Some(filter) = &mut self.state.list_filter {
                            filter.push(c);
                        }
                        self.state.selected_index = 0;
                    }
                    // Let j/k-style movement through so filter-then-pick
                    // works without leaving the bar.
                    KeyCode::Down => {
                        let _ = action_tx.send(Action::SelectNext).await;
                    }
                    KeyCode::Up => {
                        let _ = action_tx.send(Action::SelectPrevious).await;
                    }
                    _ => {}
                }
            }
            Focus::Content => {
                if keymap::focuses_search(&key) {
                    // In list views `/` filters in place; elsewhere it jumps
                    // to the Search tab.
                    if matches!(
                        self.state.view,
                        View::Queue | View::History | View::Playlists | View::PlaylistDetail
                    ) {
                        self.state.list_filter = Some(String::new());
                        self.state.focus = Focus::ListFilter;
                        self.state.selected_index = 0;
                    } else {
                        self.state.view = View::Search;
                        self.state.focus = Focus::SearchInput;
                    }
                    return;
                }
                // Esc clears a locked list filter.
                if key.code == KeyCode::Esc && self.state.list_filter.is_some() {
                    self.state.list_filter = None;
                    self.state.visible_indices = None;
                    self.state.clamp_selection();
                    return;
                }
                if self.state.view == View::NowPlaying
                    && let Some(action) = keymap::playing_pane_action(
                        &key,
                        self.state.playing_pane,
                        crate::ui::layout::Breakpoint::from_width(self.state.screen_area.width),
                    )
                {
                    let _ = action_tx.send(action).await;
                    return;
                }
                if let Some(action) = keymap::route(&key, Focus::Content, self.state.view) {
                    let _ = action_tx.send(action).await;
                }
            }
        }
    }

    /// Recompute the filtered view of the active list, and mirror the
    /// History length for the current presentation mode. Runs every loop
    /// iteration before render.
    fn sync_list_view(&mut self) {
        use crate::app::state::HistoryViewMode;

        self.state.history_len = match self.state.history_view_mode {
            HistoryViewMode::Recent => self
                .history
                .as_ref()
                .map_or(0, |history| history.recent_unique_indices().len()),
            HistoryViewMode::Top => self.history.as_ref().map_or(0, |h| h.aggregate().len()),
        };

        let filter = self
            .state
            .list_filter
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let sync_key = FilterSyncKey {
            view: self.state.view,
            history_mode: self.state.history_view_mode,
            filter: filter.clone(),
            list_revision: self.list_revision,
        };
        if self.filter_sync_key.as_ref() == Some(&sync_key) {
            return;
        }
        self.filter_sync_key = Some(sync_key);
        if filter.is_empty() {
            self.state.visible_indices = match (
                self.state.view,
                self.state.history_view_mode,
                self.history.as_ref(),
            ) {
                (View::History, HistoryViewMode::Recent, Some(history)) => {
                    Some(history.recent_unique_indices())
                }
                _ => None,
            };
            self.state.clamp_selection();
            return;
        }
        if self.state.view == View::History
            && self.state.history_view_mode == HistoryViewMode::Recent
        {
            self.state.visible_indices = self.history.as_ref().map(|history| {
                history
                    .recent_unique_indices()
                    .into_iter()
                    .filter(|&index| {
                        let entry = &history.entries()[index];
                        crate::app::filter::matches(
                            &filter,
                            &format!("{} {}", entry.artist, entry.title),
                            Some(&format!("{:?}", entry.outcome)),
                        )
                    })
                    .collect()
            });
            self.state.clamp_selection();
            return;
        }
        let rows: Vec<(String, Option<String>)> = match self.state.view {
            View::Queue => self
                .state
                .queue
                .order
                .iter()
                .map(|&i| {
                    let t = &self.state.queue.tracks[i];
                    (format!("{} {}", t.artist, t.title), None)
                })
                .collect(),
            View::Playlists => self
                .state
                .playlists
                .iter()
                .map(|p| (p.name.clone(), None))
                .collect(),
            View::PlaylistDetail => self
                .state
                .selected_playlist
                .and_then(|i| self.state.playlists.get(i))
                .map(|p| {
                    p.tracks
                        .iter()
                        .map(|t| (format!("{} {}", t.artist, t.title), None))
                        .collect()
                })
                .unwrap_or_default(),
            View::History => match (&self.history, self.state.history_view_mode) {
                (Some(history), HistoryViewMode::Recent) => history
                    .entries()
                    .iter()
                    .map(|e| {
                        (
                            format!("{} {}", e.artist, e.title),
                            Some(format!("{:?}", e.outcome)),
                        )
                    })
                    .collect(),
                (Some(history), HistoryViewMode::Top) => history
                    .aggregate()
                    .iter()
                    .map(|s| {
                        (
                            format!("{} {}", s.entry.artist, s.entry.title),
                            Some(format!("{:?}", s.entry.outcome)),
                        )
                    })
                    .collect(),
                (None, _) => Vec::new(),
            },
            _ => {
                self.state.visible_indices = None;
                return;
            }
        };
        self.state.visible_indices = Some(crate::app::filter::matching_indices(
            &filter,
            rows.iter()
                .map(|(text, outcome)| (text.clone(), outcome.as_deref())),
        ));
        self.state.clamp_selection();
    }

    /// Classify free text vs URL and dispatch accordingly (PRD 10.2).
    async fn submit_text_query(&mut self, query: String, action_tx: &mpsc::Sender<Action>) {
        let kind = classify_input(&query);
        self.state.input_kind = Some(kind.clone());
        match kind {
            InputKind::Query(q) => {
                let _ = action_tx.send(Action::SubmitSearch(q)).await;
            }
            InputKind::Video(id) => {
                let url = format!("https://www.youtube.com/watch?v={id}");
                self.autoplay_first_search = true;
                let _ = action_tx.send(Action::SubmitExactVideo(url)).await;
            }
            InputKind::Playlist(_) => {
                let _ = action_tx.send(Action::StartImport(query)).await;
                self.state.focus = Focus::Content;
            }
            InputKind::Mix(video_id) => {
                self.state.notify("Loading mix...", false);
                self.state.focus = Focus::Content;
                let yt_dlp = self.yt_dlp.clone();
                let tx = action_tx.clone();
                let ticket = self.operations.start(OperationKind::Mix);
                let operation_id = ticket.id();
                let cancellation = ticket.cancellation().clone();
                let handle = tokio::spawn(async move {
                    let action = tokio::select! {
                        () = cancellation.cancelled() => return,
                        result = yt_dlp.fetch_mix(&video_id) => match result {
                            Ok(fetch) => Action::MixLoaded {
                                operation_id,
                                title: fetch.title,
                                tracks: fetch.tracks,
                            },
                            Err(err) => Action::Notify(format!("Mix failed: {err}")),
                        }
                    };
                    let _ = tx.send(action).await;
                });
                self.operations
                    .attach(OperationKind::Mix, operation_id, handle);
            }
        }
    }

    /// Execute side effects returned by the reducer.
    async fn execute(&mut self, effects: Vec<Effect>, action_tx: &mpsc::Sender<Action>) {
        for effect in effects {
            match effect {
                Effect::RunSearch { query, generation } => {
                    self.spawn_search(query, generation, action_tx.clone());
                }
                Effect::RunExactVideo { url, generation } => {
                    self.spawn_exact_video(url, generation, action_tx.clone());
                }
                Effect::RunImport { url } => {
                    self.spawn_import(url, action_tx.clone());
                }
                Effect::ResolveAndPlay {
                    track_index_in_queue,
                } => {
                    self.state.pending_resume = None;
                    self.spawn_resolve_and_play(track_index_in_queue, action_tx);
                }
                Effect::SeekBy(seconds) => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.queue_seek_by(seconds);
                    }
                }
                Effect::SeekTo(seconds) => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.queue_seek_to(seconds);
                    }
                }
                Effect::TogglePause => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.queue_toggle_pause();
                    }
                }
                Effect::AdjustVolume(delta) => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.queue_adjust_volume(delta);
                    }
                }
                Effect::ToggleMute => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.queue_toggle_mute();
                    }
                }
                Effect::SetSpeed(speed) => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.queue_set_speed(speed);
                    }
                }
                Effect::StopPlayback => {
                    self.record_current(PlaybackOutcome::Stopped);
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.queue_stop();
                    }
                }
                Effect::QuitMpv => {
                    if let Some(p) = self.playback.as_mut() {
                        // The event loop is already exiting; wait for the
                        // final acknowledgement so graceful shutdown is not
                        // aborted when the controller is dropped.
                        let _ = p.quit().await;
                    }
                }
                Effect::PersistQueue => {
                    self.persist_queue();
                }
                Effect::PersistSession => {
                    self.maybe_save_session(self.state.playback.position_seconds, true);
                }
                Effect::PersistPlaylists | Effect::Exit => {}
            }
        }
    }

    /// Actions that need services (playlist storage, history) or full state.
    async fn handle_service_action(&mut self, action: Action, action_tx: &mpsc::Sender<Action>) {
        match action {
            Action::OpenInBrowser => {
                let track = match self.state.view {
                    View::Search => self.resolve_selected_track(),
                    View::NowPlaying => self.state.current_track.clone(),
                    _ => None,
                };
                match track {
                    Some(track) => match open_browser(&track.webpage_url) {
                        Ok(()) => self.state.notify("Opened in browser", false),
                        Err(error) => self
                            .state
                            .notify(&format!("Couldn't open browser: {error}"), true),
                    },
                    None => self.state.notify("No track selected", true),
                }
            }
            Action::ResumeTrack {
                track,
                position_seconds,
            } => {
                let queue_position = self
                    .state
                    .queue
                    .order
                    .iter()
                    .position(|&index| self.state.queue.tracks[index].id == track.id)
                    .unwrap_or_else(|| {
                        self.state.queue.push(track.clone());
                        self.state.queue.order.len() - 1
                    });
                self.state.queue.position = Some(queue_position);
                self.state.current_track = Some(track.clone());
                self.state.playback.position_seconds = position_seconds;
                self.state.playback.duration_seconds =
                    track.duration_seconds.map(|value| value as f64);
                self.state.pending_resume = Some(crate::app::state::PendingResume {
                    track: track.clone(),
                    position_seconds,
                    armed: false,
                    play_on_load: true,
                });
                self.execute(vec![Effect::PersistQueue], action_tx).await;
                self.spawn_pending_resume_resolution(track, action_tx);
            }
            Action::PlaybackResolved { track_id, url, .. } => {
                let Some(track) = self
                    .state
                    .current_track
                    .clone()
                    .filter(|track| track.id == track_id)
                else {
                    return;
                };
                let loaded = match self.playback.as_mut() {
                    Some(playback) => playback.queue_load(&url, &track.title),
                    None => {
                        self.state.notify("mpv not running", true);
                        return;
                    }
                };
                if let Err(err) = loaded {
                    self.state.notify(&format!("mpv: {err}"), true);
                } else {
                    self.spawn_details_fetch(&track, action_tx);
                    self.spawn_thumbnail_fetch(&track, ThumbnailPurpose::CurrentTrack, action_tx);
                }
            }
            Action::PlaybackResolveFailed {
                queue_position,
                track_id,
                ..
            } => {
                let still_requested = self
                    .state
                    .queue
                    .order
                    .get(queue_position)
                    .and_then(|index| self.state.queue.tracks.get(*index))
                    .is_some_and(|track| track.id == track_id);
                if still_requested && self.config.playback.continue_on_error {
                    let _ = action_tx.send(Action::NextTrack).await;
                }
            }
            Action::PlaybackEvent(PlaybackEvent::Started) => {
                self.after_track_started(action_tx);
            }
            Action::PrefetchResolved { track_id, url, .. } => {
                self.prefetched = Some((track_id, url, Instant::now()));
            }
            Action::RadioTracksLoaded { .. } => {
                self.radio_fetching = false;
                // The reducer already appended; prefetch the fresh next track.
                self.after_track_started(action_tx);
            }
            Action::ToggleRadio if self.state.radio => {
                // Radio switched on with a finished queue: refill right away.
                if self.state.queue.position.is_none() || self.state.current_track.is_none() {
                    let seed = self
                        .state
                        .current_track
                        .as_ref()
                        .map(|t| t.id.clone())
                        .or_else(|| {
                            self.state
                                .queue
                                .order
                                .last()
                                .map(|&i| self.state.queue.tracks[i].id.clone())
                        })
                        .or_else(|| {
                            self.history
                                .as_ref()
                                .and_then(|h| h.entries().first())
                                .map(|e| e.track_id.clone())
                        });
                    if let Some(seed) = seed {
                        self.spawn_radio_refill(seed, action_tx);
                    } else {
                        self.state
                            .notify("Radio needs a seed — play something first", false);
                    }
                }
            }
            Action::SessionStreamResolved { track_id, url, .. } => {
                let Some(pending) = self.state.pending_resume.as_mut() else {
                    return;
                };
                if pending.track.id != track_id {
                    return;
                }
                let paused = !pending.play_on_load;
                let title = pending.track.title.clone();
                let position = pending.position_seconds;
                let track = pending.track.clone();
                let loaded = match self.playback.as_mut() {
                    Some(p) => p.queue_load_at(&url, &title, Some(position), paused),
                    None => return,
                };
                match loaded {
                    Ok(()) => {
                        if paused {
                            if let Some(pending) = self.state.pending_resume.as_mut() {
                                pending.armed = true;
                            }
                        } else {
                            self.state.pending_resume = None;
                        }
                        self.spawn_details_fetch(&track, action_tx);
                        self.spawn_thumbnail_fetch(
                            &track,
                            ThumbnailPurpose::CurrentTrack,
                            action_tx,
                        );
                    }
                    Err(err) => {
                        self.state.pending_resume = None;
                        self.state.notify(&format!("Resume failed: {err}"), true);
                    }
                }
            }
            Action::SessionResolveFailed {
                track_id, message, ..
            } => {
                if self
                    .state
                    .pending_resume
                    .as_ref()
                    .is_some_and(|p| p.track.id == track_id)
                {
                    self.state.pending_resume = None;
                    if self.state.current_track.as_ref().map(|t| t.id.as_str())
                        == Some(track_id.as_str())
                    {
                        self.state.current_track = None;
                    }
                    tracing::warn!(%message, "session resume resolve failed");
                    self.state.notify("Couldn't resume the last session", true);
                }
            }
            Action::SearchCompleted { .. } if self.autoplay_first_search => {
                self.autoplay_first_search = false;
                if self.state.active_list_len() > 0 {
                    self.state.selected_index = 0;
                    let _ = action_tx.send(Action::PlaySelected).await;
                }
            }
            Action::PlaySelected if self.state.view == View::Home => {
                use crate::app::state::HomeSection;
                match self.state.home_section {
                    HomeSection::Resume => {
                        let _ = action_tx.send(Action::PlayPause).await;
                    }
                    HomeSection::Recent => {
                        if let Some(track) = self.resolve_selected_track() {
                            let _ = action_tx.send(Action::PlayTrack(track)).await;
                        }
                    }
                    HomeSection::Playlists => {
                        if let Some(id) = self.selected_playlist_id() {
                            let _ = action_tx.send(Action::LoadPlaylistIntoQueue(id)).await;
                        }
                    }
                }
            }
            Action::AddSelectedToQueue | Action::AddSelectedAsNext => {
                if let Some(track) = self.resolve_selected_track() {
                    let next = if matches!(action, Action::AddSelectedAsNext) {
                        Action::AddNext(track)
                    } else {
                        Action::AddToQueue(track)
                    };
                    let _ = action_tx.send(next).await;
                }
            }
            Action::PlaySelected if self.state.view == View::History => {
                if let Some(track) = self.resolve_selected_track() {
                    let _ = action_tx.send(Action::PlayTrack(track)).await;
                }
            }
            Action::OpenPlaylistPicker => {
                if let Some(track) = self.resolve_selected_track() {
                    self.state.picker = Some(crate::app::state::PickerState {
                        track,
                        filter: String::new(),
                        selected: 0,
                    });
                }
            }
            Action::PickerSubmit => {
                self.submit_picker().await;
            }
            Action::DeleteSelectedHistoryEntry => {
                use crate::app::state::HistoryViewMode;
                if self.state.history_view_mode != HistoryViewMode::Recent {
                    self.state
                        .notify("Switch to recent (g) to delete entries", false);
                    return;
                }
                let index = self.state.resolve_index(self.state.selected_index);
                if let Some(history) = self.history.as_mut() {
                    history.remove(index);
                    self.state.history_len = history.recent_unique_indices().len();
                    self.state.clamp_selection();
                }
                self.list_revision = self.list_revision.wrapping_add(1);
                self.persist_history();
            }
            Action::RemoveSelectedFromPlaylist => {
                if self.state.view != View::PlaylistDetail {
                    return;
                }
                let index = self.state.resolve_index(self.state.selected_index);
                let Some(playlist) = self
                    .state
                    .selected_playlist
                    .and_then(|i| self.state.playlists.get_mut(i))
                else {
                    return;
                };
                if index >= playlist.tracks.len() {
                    return;
                }
                playlist.tracks.remove(index);
                playlist.updated_at = chrono::Utc::now();
                let snapshot = playlist.clone();
                match self.playlists.save(&snapshot) {
                    Ok(()) => self.state.notify("Removed from playlist", false),
                    Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
                }
                self.state.clamp_selection();
            }
            Action::PlaylistEditorSubmit => {
                let Some(editor) = self.state.playlist_editor.clone() else {
                    return;
                };
                let name = editor.name.trim();
                if name.is_empty() {
                    self.state.notify("Playlist name is required", true);
                    return;
                }
                let Some(playlist) = self
                    .state
                    .selected_playlist
                    .and_then(|index| self.state.playlists.get_mut(index))
                else {
                    self.state.playlist_editor = None;
                    return;
                };
                let previous = playlist.clone();
                playlist.name = name.to_string();
                playlist.description = editor.description.trim().to_string();
                playlist.updated_at = chrono::Utc::now();
                match self.playlists.save(playlist) {
                    Ok(()) => {
                        self.state.playlist_editor = None;
                        self.state.notify("Playlist details saved", false);
                    }
                    Err(error) => {
                        *playlist = previous;
                        self.state.notify(&format!("Save failed: {error}"), true);
                    }
                }
            }
            Action::MoveSelectedInPlaylist(delta) => {
                if self.state.view != View::PlaylistDetail {
                    return;
                }
                if self.state.visible_indices.is_some() {
                    self.state
                        .notify("Clear the filter (Esc) to reorder", false);
                    return;
                }
                let from = self.state.selected_index;
                let Some(playlist) = self
                    .state
                    .selected_playlist
                    .and_then(|i| self.state.playlists.get_mut(i))
                else {
                    return;
                };
                let len = playlist.tracks.len();
                if len < 2 || from >= len {
                    return;
                }
                let to = from.saturating_add_signed(delta as isize).min(len - 1);
                if from == to {
                    return;
                }
                let track = playlist.tracks.remove(from);
                playlist.tracks.insert(to, track);
                playlist.updated_at = chrono::Utc::now();
                let snapshot = playlist.clone();
                self.state.selected_index = to;
                if let Err(err) = self.playlists.save(&snapshot) {
                    self.state.notify(&format!("Save failed: {err}"), true);
                }
            }
            Action::LoadSelectedPlaylistIntoQueue | Action::AppendSelectedPlaylistToQueue => {
                if let Some(id) = self.selected_playlist_id() {
                    let next = if matches!(action, Action::LoadSelectedPlaylistIntoQueue) {
                        Action::LoadPlaylistIntoQueue(id)
                    } else {
                        Action::AppendPlaylistToQueue(id)
                    };
                    let _ = action_tx.send(next).await;
                }
            }
            Action::DeleteSelectedPlaylist => {
                if let Some(id) = self.selected_playlist_id() {
                    let _ = action_tx.send(Action::DeletePlaylist(id)).await;
                }
            }
            Action::LoadPlaylistIntoQueue(id) => {
                if let Some(tracks) = self.playlist_tracks(&id) {
                    self.state.queue.load_tracks(tracks);
                    if !self.state.queue.order.is_empty() {
                        self.state.queue.position = Some(0);
                        self.state.current_track = self.state.queue.current().cloned();
                        self.execute(
                            vec![
                                Effect::ResolveAndPlay {
                                    track_index_in_queue: 0,
                                },
                                Effect::PersistQueue,
                            ],
                            action_tx,
                        )
                        .await;
                    }
                }
            }
            Action::AppendPlaylistToQueue(id) => {
                if let Some(tracks) = self.playlist_tracks(&id) {
                    for track in tracks {
                        self.state.queue.push(track);
                    }
                    self.state.notify("Playlist appended to queue", false);
                    self.execute(vec![Effect::PersistQueue], action_tx).await;
                }
            }
            Action::DeletePlaylistConfirmed(id) => match self.playlists.delete(&id) {
                Ok(()) => {
                    self.state.playlists.retain(|p| p.id != id);
                    self.state.selected_playlist = None;
                    self.state.clamp_selection();
                    self.state.notify("Playlist deleted", false);
                }
                Err(err) => self.state.notify(&format!("Delete failed: {err}"), true),
            },
            Action::ConfirmYes => {
                if let Some(confirm) = self.state.confirm.take() {
                    let action = *confirm.action;
                    let _ = action_tx.send(action).await;
                }
            }
            Action::PromptSubmit => {
                if let Some(prompt) = self.state.prompt.take() {
                    let text = prompt.buffer.trim().to_string();
                    match (prompt.purpose, text.is_empty()) {
                        (purpose, true) => {
                            let required = match purpose {
                                PromptPurpose::ImportPlaylistUrl => "A URL is required",
                                PromptPurpose::ImportPlaylistJson => "Playlist JSON is required",
                                _ => "A playlist name is required",
                            };
                            self.state.notify(required, true);
                        }
                        (PromptPurpose::SaveQueueAsPlaylist, false) => {
                            self.save_queue_as_playlist(&text);
                        }
                        (PromptPurpose::NewPlaylist, false) => {
                            self.create_playlist(&text);
                        }
                        (PromptPurpose::RenamePlaylist, false) => {
                            self.rename_selected_playlist(&text);
                        }
                        (PromptPurpose::ImportPlaylistUrl, false) => {
                            let _ = action_tx.send(Action::StartImport(text)).await;
                        }
                        (PromptPurpose::ImportPlaylistJson, false) => {
                            match crate::playlists::import::parse_pasted_json(&text) {
                                Ok(playlists) => self.save_json_playlists(playlists),
                                Err(message) => {
                                    self.state.prompt = Some(crate::app::state::PromptState {
                                        purpose: PromptPurpose::ImportPlaylistJson,
                                        buffer: text,
                                    });
                                    self.state.notify(&message, true);
                                }
                            }
                        }
                    }
                }
            }
            Action::ConfirmImport => {
                if let Some(ImportState::Review { playlist, .. }) = self.state.import.take() {
                    match self.playlists.save(&playlist) {
                        Ok(()) => {
                            let saved = *playlist;
                            self.state
                                .activity
                                .push(crate::history::activity::ActivityEvent::new(
                                    crate::history::activity::ActivityKind::PlaylistImported,
                                    saved.name.clone(),
                                    format!("{} tracks", saved.tracks.len()),
                                ));
                            self.maybe_save_session(self.state.playback.position_seconds, true);
                            self.state
                                .notify(&format!("Imported \"{}\"", saved.name), false);
                            let _ = action_tx.send(Action::PlaylistSaved(saved)).await;
                        }
                        Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
                    }
                }
            }
            Action::ClearHistoryConfirmed => {
                if let Some(history) = self.history.as_mut() {
                    history.clear();
                }
                self.persist_history();
                self.state.notify("History cleared", false);
            }
            _ => {}
        }
    }

    /// Resolve the currently selected track across track-listing views,
    /// mapping through the in-list filter and History presentation mode.
    fn resolve_selected_track(&self) -> Option<Track> {
        let index = self.state.resolve_index(self.state.selected_index);
        match self.state.view {
            View::Home => match self.state.home_section {
                crate::app::state::HomeSection::Recent => self.history.as_ref().and_then(|h| {
                    h.recent_unique(self.state.selected_index + 1)
                        .into_iter()
                        .nth(self.state.selected_index)
                }),
                crate::app::state::HomeSection::Resume
                | crate::app::state::HomeSection::Playlists => None,
            },
            View::History => match self.state.history_view_mode {
                crate::app::state::HistoryViewMode::Recent => self
                    .history
                    .as_ref()
                    .and_then(|h| h.entries().get(index).map(|e| e.to_track())),
                crate::app::state::HistoryViewMode::Top => self
                    .history
                    .as_ref()
                    .and_then(|h| h.aggregate().get(index).map(|s| s.entry.to_track())),
            },
            View::Search => match &self.state.search {
                crate::media::search::SearchState::Results { tracks, .. } => {
                    tracks.get(index).cloned()
                }
                _ => None,
            },
            View::Queue => self
                .state
                .queue
                .order
                .get(index)
                .map(|&i| self.state.queue.tracks[i].clone()),
            View::PlaylistDetail => self
                .state
                .selected_playlist
                .and_then(|i| self.state.playlists.get(i))
                .and_then(|p| p.tracks.get(index))
                .map(Track::from),
            _ => None,
        }
    }

    /// Apply the add-to-playlist picker: add to the chosen playlist, or
    /// create one named after the filter text first.
    async fn submit_picker(&mut self) {
        let Some(picker) = self.state.picker.take() else {
            return;
        };
        let (create_new, matching) =
            crate::app::filter::picker_candidates(&self.state.playlists, &picker.filter);

        let target_index = if create_new {
            if picker.selected == 0 {
                // Create a playlist named after the typed filter, then add.
                let playlist = Playlist::new(picker.filter.trim());
                match self.playlists.save(&playlist) {
                    Ok(()) => {
                        self.state.playlists.push(playlist);
                        Some(self.state.playlists.len() - 1)
                    }
                    Err(err) => {
                        self.state.notify(&format!("Save failed: {err}"), true);
                        return;
                    }
                }
            } else {
                matching.get(picker.selected - 1).copied()
            }
        } else {
            matching.get(picker.selected).copied()
        };
        let Some(target_index) = target_index else {
            return;
        };
        let Some(playlist) = self.state.playlists.get_mut(target_index) else {
            return;
        };

        if playlist.tracks.iter().any(|t| t.id == picker.track.id) {
            let name = playlist.name.clone();
            self.state.notify(&format!("Already in \"{name}\""), false);
            return;
        }
        playlist
            .tracks
            .push(crate::playlists::model::PlaylistTrack::from(&picker.track));
        playlist.updated_at = chrono::Utc::now();
        let snapshot = playlist.clone();
        match self.playlists.save(&snapshot) {
            Ok(()) => {
                self.state
                    .activity
                    .push(crate::history::activity::ActivityEvent::new(
                        crate::history::activity::ActivityKind::AddedToPlaylist,
                        picker.track.title.clone(),
                        snapshot.name.clone(),
                    ));
                self.state
                    .notify(&format!("Added to \"{}\"", snapshot.name), false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
        self.state.sort_playlists_by_updated();
        self.maybe_save_session(self.state.playback.position_seconds, true);
    }

    /// Playlist ID relevant to the current view and selection.
    fn selected_playlist_id(&self) -> Option<String> {
        match self.state.view {
            View::Home if self.state.home_section == crate::app::state::HomeSection::Playlists => {
                self.state
                    .playlists
                    .get(self.state.selected_index)
                    .map(|p| p.id.clone())
            }
            View::Playlists => self
                .state
                .playlists
                .get(self.state.selected_index)
                .map(|p| p.id.clone()),
            View::PlaylistDetail => self
                .state
                .selected_playlist
                .and_then(|i| self.state.playlists.get(i))
                .map(|p| p.id.clone()),
            _ => None,
        }
    }

    fn playlist_tracks(&self, id: &str) -> Option<Vec<Track>> {
        self.state
            .playlists
            .iter()
            .find(|p| p.id == id)
            .map(Playlist::to_tracks)
    }

    fn save_queue_as_playlist(&mut self, name: &str) {
        if self.state.queue.tracks.is_empty() {
            self.state.notify("Queue is empty", true);
            return;
        }
        let mut playlist = Playlist::new(name);
        playlist.tracks = self
            .state
            .queue
            .tracks
            .iter()
            .map(crate::playlists::model::PlaylistTrack::from)
            .collect();
        match self.playlists.save(&playlist) {
            Ok(()) => {
                self.state.playlists.push(playlist);
                self.state.sort_playlists_by_updated();
                self.state.notify("Queue saved as playlist", false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
    }

    fn create_playlist(&mut self, name: &str) {
        let playlist = Playlist::new(name);
        match self.playlists.save(&playlist) {
            Ok(()) => {
                self.state.playlists.push(playlist);
                self.state.sort_playlists_by_updated();
                self.state.notify("Playlist created", false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
    }

    fn save_json_playlists(&mut self, playlists: Vec<Playlist>) {
        let requested_playlist_count = playlists.len();
        let requested_track_count = playlists
            .iter()
            .map(|playlist| playlist.tracks.len())
            .sum::<usize>();
        let mut staged: Vec<Playlist> = Vec::with_capacity(requested_playlist_count);
        let mut added_tracks = 0usize;
        for incoming in playlists {
            let matching_staged = staged.iter().position(|playlist| {
                playlist
                    .name
                    .trim()
                    .eq_ignore_ascii_case(incoming.name.trim())
            });
            let matching_existing = self.state.playlists.iter().find(|playlist| {
                playlist
                    .name
                    .trim()
                    .eq_ignore_ascii_case(incoming.name.trim())
            });
            if let Some(index) = matching_staged {
                added_tracks += merge_playlist_tracks(&mut staged[index], incoming);
            } else if let Some(existing) = matching_existing {
                let mut merged = existing.clone();
                added_tracks += merge_playlist_tracks(&mut merged, incoming);
                staged.push(merged);
            } else {
                added_tracks += incoming.tracks.len();
                staged.push(incoming);
            }
        }
        let originals = staged
            .iter()
            .filter_map(|staged_playlist| {
                self.state
                    .playlists
                    .iter()
                    .find(|playlist| playlist.id == staged_playlist.id)
                    .cloned()
            })
            .collect::<Vec<_>>();
        let mut saved_ids: Vec<String> = Vec::with_capacity(staged.len());
        for playlist in &staged {
            if let Err(error) = self.playlists.save(playlist) {
                for id in &saved_ids {
                    let rollback = originals
                        .iter()
                        .find(|playlist| playlist.id == *id)
                        .map_or_else(
                            || self.playlists.delete(id),
                            |playlist| self.playlists.save(playlist),
                        );
                    if let Err(rollback_error) = rollback {
                        tracing::error!(?rollback_error, playlist_id = %id, "JSON import rollback failed");
                    }
                }
                self.state.notify(
                    &format!("JSON import failed; no playlists kept: {error}"),
                    true,
                );
                return;
            }
            saved_ids.push(playlist.id.clone());
        }
        for playlist in staged {
            match self
                .state
                .playlists
                .iter()
                .position(|existing| existing.id == playlist.id)
            {
                Some(index) => self.state.playlists[index] = playlist,
                None => self.state.playlists.push(playlist),
            }
        }
        self.state.sort_playlists_by_updated();
        self.state
            .activity
            .push(crate::history::activity::ActivityEvent::new(
                crate::history::activity::ActivityKind::PlaylistImported,
                format!("{requested_playlist_count} JSON playlists"),
                format!("{added_tracks} new tracks"),
            ));
        self.maybe_save_session(self.state.playback.position_seconds, true);
        self.state.notify(
            &format!(
                "Imported {added_tracks} new tracks · {} duplicates skipped",
                requested_track_count.saturating_sub(added_tracks)
            ),
            false,
        );
    }

    fn rename_selected_playlist(&mut self, name: &str) {
        let Some(id) = self.selected_playlist_id() else {
            return;
        };
        let Some(playlist) = self.state.playlists.iter_mut().find(|p| p.id == id) else {
            return;
        };
        playlist.name = name.to_string();
        playlist.updated_at = chrono::Utc::now();
        match self.playlists.save(playlist) {
            Ok(()) => self.state.notify("Playlist renamed", false),
            Err(err) => self.state.notify(&format!("Rename failed: {err}"), true),
        }
        self.state.sort_playlists_by_updated();
    }

    /// After a track starts: prefetch the next track's stream URL, and in
    /// radio mode top the queue up when playing the last queued track.
    fn after_track_started(&mut self, action_tx: &mpsc::Sender<Action>) {
        let len = self.state.queue.order.len();
        let Some(pos) = self.state.queue.position else {
            return;
        };

        if self.state.radio
            && pos + 1 >= len
            && !self.radio_fetching
            && let Some(track) = self.state.current_track.clone()
        {
            self.spawn_radio_refill(track.id, action_tx);
        }

        // Prefetch the next track (pointless when repeating the current one).
        if self.state.queue.repeat == crate::queue::RepeatMode::Track {
            return;
        }
        let next_pos = if pos + 1 < len {
            Some(pos + 1)
        } else if self.state.queue.repeat == crate::queue::RepeatMode::Queue && len > 0 {
            Some(0)
        } else {
            None
        };
        let Some(next_track) = next_pos
            .and_then(|p| self.state.queue.order.get(p))
            .map(|&i| self.state.queue.tracks[i].clone())
        else {
            return;
        };
        if self
            .prefetched
            .as_ref()
            .is_some_and(|(id, _, _)| *id == next_track.id)
        {
            return;
        }
        let yt_dlp = self.yt_dlp.clone();
        let tx = action_tx.clone();
        let ticket = self.operations.start(OperationKind::Prefetch);
        let operation_id = ticket.id();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            // Failures are silent: playback falls back to resolving on demand.
            let result = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.resolve_stream(&next_track.webpage_url) => result,
            };
            if let Ok(url) = result {
                let _ = tx
                    .send(Action::PrefetchResolved {
                        operation_id,
                        track_id: next_track.id,
                        url,
                    })
                    .await;
            }
        });
        self.operations
            .attach(OperationKind::Prefetch, operation_id, handle);
    }

    /// Fetch more tracks from YouTube's mix for `seed_id` (radio mode).
    fn spawn_radio_refill(&mut self, seed_id: String, action_tx: &mpsc::Sender<Action>) {
        self.radio_fetching = true;
        let ticket = self.operations.start(OperationKind::Radio);
        let operation_id = ticket.id();
        let _ = reduce(&mut self.state, Action::RadioRefillStarted { operation_id });
        let yt_dlp = self.yt_dlp.clone();
        let tx = action_tx.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_mix(&seed_id) => match result {
                    Ok(fetch) => Action::RadioTracksLoaded {
                        operation_id,
                        tracks: fetch.tracks,
                    },
                    Err(err) => {
                        tracing::warn!(?err, "radio refill failed");
                        Action::RadioTracksLoaded {
                            operation_id,
                            tracks: Vec::new(),
                        }
                    }
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Radio, operation_id, handle);
    }

    /// Fire an async yt-dlp search; completion arrives as an action.
    fn spawn_search(&mut self, query: String, generation: u64, action_tx: mpsc::Sender<Action>) {
        let ticket = self.operations.start(OperationKind::Search);
        let operation_id = ticket.id();
        let yt_dlp = self.yt_dlp.clone();
        let limit = self.config.search.result_limit;
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.search(&query, limit) => match result {
                    Ok(tracks) => Action::SearchCompleted { generation, tracks },
                    Err(err) => Action::SearchFailed {
                        generation,
                        message: err.to_string(),
                    },
                }
            };
            let _ = action_tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Search, operation_id, handle);
    }

    fn spawn_exact_video(&mut self, url: String, generation: u64, action_tx: mpsc::Sender<Action>) {
        let ticket = self.operations.start(OperationKind::Search);
        let operation_id = ticket.id();
        let yt_dlp = self.yt_dlp.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_video(&url) => match result {
                    Ok(track) => Action::SearchCompleted {
                        generation,
                        tracks: vec![track],
                    },
                    Err(err) => Action::SearchFailed {
                        generation,
                        message: err.to_string(),
                    },
                }
            };
            let _ = action_tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Search, operation_id, handle);
    }

    /// Fire an async playlist import; completion arrives as an action.
    fn spawn_import(&mut self, url: String, action_tx: mpsc::Sender<Action>) {
        let ticket = self.operations.start(OperationKind::Import);
        let operation_id = ticket.id();
        let _ = reduce(
            &mut self.state,
            Action::ImportStarted {
                operation_id,
                url: url.clone(),
            },
        );
        let yt_dlp = self.yt_dlp.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_playlist(&url) => match result {
                    Ok(fetch) => Action::ImportCompleted {
                        operation_id,
                        url,
                        title: fetch.title,
                        remote_id: fetch.remote_id,
                        tracks: fetch.tracks,
                        rejections: fetch.rejections,
                    },
                    Err(err) => Action::ImportFailed {
                        operation_id,
                        url,
                        message: err.to_string(),
                    },
                }
            };
            let _ = action_tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Import, operation_id, handle);
    }

    /// Resolve a persisted resume target through the existing session flow.
    fn spawn_pending_resume_resolution(&mut self, track: Track, action_tx: &mpsc::Sender<Action>) {
        let yt_dlp = self.yt_dlp.clone();
        let tx = action_tx.clone();
        let ticket = self.operations.start(OperationKind::Session);
        let operation_id = ticket.id();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.resolve_stream(&track.webpage_url) => match result {
                    Ok(url) => Action::SessionStreamResolved {
                        operation_id,
                        track_id: track.id.clone(),
                        url,
                    },
                    Err(error) => Action::SessionResolveFailed {
                        operation_id,
                        track_id: track.id.clone(),
                        message: error.to_string(),
                    },
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Session, operation_id, handle);
    }

    /// Track playback lifecycle for history recording (PRD 10.11) and the
    /// resume-session snapshot.
    fn on_playback_event(&mut self, event: &PlaybackEvent) {
        match event {
            PlaybackEvent::Started => {
                self.listening.started();
                self.maybe_save_session(0.0, true);
            }
            PlaybackEvent::PositionChanged(position) => {
                self.listening.position(*position);
                self.maybe_save_session(*position, false);
            }
            PlaybackEvent::PauseChanged(paused) => {
                self.listening.paused(*paused);
                if *paused {
                    self.capture_resume_point();
                }
            }
            PlaybackEvent::EndFile { reason } => {
                self.capture_resume_point();
                let outcome = if reason == "error" {
                    PlaybackOutcome::Failed
                } else {
                    PlaybackOutcome::Completed
                };
                self.record_current(outcome);
            }
            _ => {}
        }
    }

    /// Append the outgoing track to history with the given outcome.
    fn record_current(&mut self, outcome: PlaybackOutcome) {
        let Some(track) = self.state.current_track.clone() else {
            let _ = self.listening.finish();
            return;
        };
        let listened = self.listening.finish();
        let Some(history) = self.history.as_mut() else {
            return;
        };
        history.record(HistoryEntry::from_track(&track, None, outcome, listened));
        self.list_revision = self.list_revision.wrapping_add(1);
        self.filter_sync_key = None;
        self.persist_history();
    }

    fn capture_resume_point(&mut self) {
        let Some(track) = &self.state.current_track else {
            return;
        };
        let Some(duration) = self
            .state
            .playback
            .duration_seconds
            .or_else(|| track.duration_seconds.map(|value| value as f64))
        else {
            return;
        };
        if self.state.resume_points.record(
            track.id.clone(),
            self.state.playback.position_seconds,
            duration,
            chrono::Utc::now(),
        ) {
            self.maybe_save_session(self.state.playback.position_seconds, true);
        }
    }

    fn submit_persistence(
        &mut self,
        key: &str,
        description: &str,
        job: impl FnOnce() -> Result<()> + Send + 'static,
    ) {
        let Some(writer) = &self.persistence_writer else {
            if let Err(err) = job() {
                self.state.notify(
                    &format!("Could not save {description}: {err}. Changes are not durable."),
                    true,
                );
            }
            return;
        };
        if let Err(err) = writer.submit(key, description, job) {
            self.state
                .notify(&format!("Could not queue {description} save: {err}"), true);
        }
    }

    fn persist_queue(&mut self) {
        let path = self.paths.queue_file();
        let queue = self.state.queue.clone();
        self.submit_persistence("queue", "queue", move || {
            crate::queue::service::save(&path, &queue)
        });
    }

    fn persist_history(&mut self) {
        if let Some(history) = self.history.clone() {
            self.submit_persistence("history", "history", move || history.save());
        }
    }

    /// Resolve the track at a queue position and start playback (PRD 10.4).
    ///
    /// Resolution retries once in an owned task. A matching failure action
    /// advances the queue when `continueOnError` is enabled.
    fn spawn_resolve_and_play(&mut self, queue_position: usize, action_tx: &mpsc::Sender<Action>) {
        let Some(track) = self
            .state
            .queue
            .order
            .get(queue_position)
            .and_then(|index| self.state.queue.tracks.get(*index))
            .cloned()
        else {
            return;
        };
        let prefetched = self.prefetched.take().and_then(|(id, url, at)| {
            (id == track.id && at.elapsed() < Duration::from_secs(2 * 3600)).then_some(url)
        });
        let ticket = self.operations.start(OperationKind::Playback);
        let operation_id = ticket.id();
        let _ = reduce(
            &mut self.state,
            Action::PlaybackResolveStarted {
                operation_id,
                queue_position,
                track_id: track.id.clone(),
            },
        );
        self.state
            .notify(&format!("Resolving: {}", track.title), false);
        let yt_dlp = self.yt_dlp.clone();
        let tx = action_tx.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let resolve = async {
                if let Some(url) = prefetched {
                    return Ok(url);
                }
                match yt_dlp.resolve_stream(&track.webpage_url).await {
                    Ok(url) => Ok(url),
                    Err(first_err) => {
                        tracing::warn!(?first_err, track = %track.id, "resolve failed; retrying once");
                        yt_dlp.resolve_stream(&track.webpage_url).await
                    }
                }
            };
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = resolve => match result {
                    Ok(url) => Action::PlaybackResolved {
                        operation_id,
                        queue_position,
                        track_id: track.id,
                        url,
                    },
                    Err(err) => Action::PlaybackResolveFailed {
                        operation_id,
                        queue_position,
                        track_id: track.id,
                        message: err.to_string(),
                    },
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Playback, operation_id, handle);
    }

    /// Fetch extended metadata in the background for the now-playing view.
    fn spawn_details_fetch(&mut self, track: &Track, action_tx: &mpsc::Sender<Action>) {
        let ticket = self.operations.start(OperationKind::Details);
        let operation_id = ticket.id();
        let yt_dlp = self.yt_dlp.clone();
        let url = track.webpage_url.clone();
        let track_id = track.id.clone();
        let _ = reduce(
            &mut self.state,
            Action::DetailsStarted {
                operation_id,
                track_id: track_id.clone(),
            },
        );
        let tx = action_tx.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_details(&url) => match result {
                    Ok(details) => Action::DetailsLoaded {
                        operation_id,
                        track_id,
                        details: Box::new(details),
                    },
                    Err(err) => Action::DetailsFailed {
                        operation_id,
                        track_id,
                        message: err.to_string(),
                    },
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Details, operation_id, handle);
    }

    /// Fetch the track's YouTube thumbnail in the background.
    ///
    /// Uses the deterministic i.ytimg.com URL; downloaded via curl so no
    /// HTTP client dependency is needed. Failures are silent — the UI
    /// simply shows no image.
    fn spawn_thumbnail_fetch(
        &mut self,
        track: &Track,
        purpose: ThumbnailPurpose,
        action_tx: &mpsc::Sender<Action>,
    ) {
        const MAX_THUMBNAIL_BYTES: usize = 5 * 1024 * 1024;
        let operation_kind = purpose.operation_kind();
        let ticket = self.operations.start(operation_kind);
        let operation_id = ticket.id();
        let track_id = track.id.clone();
        let tx = action_tx.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let url = format!("https://i.ytimg.com/vi/{track_id}/hqdefault.jpg");
            let output = tokio::select! {
                () = cancellation.cancelled() => return,
                output = async {
                    let mut child = tokio::process::Command::new("curl")
                        .args([
                            "-sfL",
                            "--max-time",
                            "15",
                            "--max-filesize",
                            &MAX_THUMBNAIL_BYTES.to_string(),
                            "--",
                            &url,
                        ])
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::null())
                        .kill_on_drop(true)
                        .spawn()?;
                    let mut bytes = Vec::new();
                    child
                        .stdout
                        .take()
                        .expect("curl stdout was piped")
                        .take((MAX_THUMBNAIL_BYTES + 1) as u64)
                        .read_to_end(&mut bytes)
                        .await?;
                    let status = child.wait().await?;
                    Ok::<_, std::io::Error>((status, bytes))
                } => output,
            };
            if let Ok((status, bytes)) = output
                && status.success()
                && !bytes.is_empty()
                && bytes.len() <= MAX_THUMBNAIL_BYTES
            {
                let action = match purpose {
                    ThumbnailPurpose::CurrentTrack => Action::ThumbnailLoaded {
                        operation_id,
                        track_id,
                        bytes,
                    },
                    ThumbnailPurpose::SearchSelection => Action::SearchThumbnailLoaded {
                        operation_id,
                        track_id,
                        bytes,
                    },
                };
                let _ = tx.send(action).await;
            }
        });
        self.operations.attach(operation_kind, operation_id, handle);
    }

    /// Decode downloaded thumbnail bytes into a resize protocol.
    fn on_thumbnail_loaded(&mut self, track_id: String, bytes: Vec<u8>) {
        if self.state.current_track.as_ref().map(|t| t.id.as_str()) != Some(track_id.as_str()) {
            return;
        }
        match crate::media::decode_thumbnail(&bytes) {
            Ok(dyn_img) => {
                self.state.thumbnail = Some(self.picker.new_resize_protocol(dyn_img));
            }
            Err(err) => tracing::warn!(?err, "thumbnail decode failed"),
        }
    }

    /// Start or reuse the thumbnail preview for the selected Search result.
    fn sync_search_thumbnail(&mut self, action_tx: &mpsc::Sender<Action>) {
        if self.state.view != View::Search {
            return;
        }
        let selected = match &self.state.search {
            crate::media::search::SearchState::Results { tracks, .. } => {
                tracks.get(self.state.selected_index).cloned()
            }
            _ => None,
        };
        let Some(track) = selected else {
            self.state.search_thumbnail_track_id = None;
            self.state.search_thumbnail = None;
            self.operations.cancel(OperationKind::SearchThumbnail);
            return;
        };
        if self.state.search_thumbnail_track_id.as_deref() == Some(track.id.as_str()) {
            return;
        }
        self.state.search_thumbnail_track_id = Some(track.id.clone());
        self.state.search_thumbnail = None;
        self.spawn_thumbnail_fetch(&track, ThumbnailPurpose::SearchSelection, action_tx);
    }

    /// Decode a selected-result thumbnail only if that result is still active.
    fn on_search_thumbnail_loaded(&mut self, track_id: String, bytes: Vec<u8>) {
        if self.state.search_thumbnail_track_id.as_deref() != Some(track_id.as_str()) {
            return;
        }
        match crate::media::decode_thumbnail(&bytes) {
            Ok(image) => {
                self.state.search_thumbnail = Some(self.picker.new_resize_protocol(image));
            }
            Err(err) => tracing::warn!(?err, "search thumbnail decode failed"),
        }
    }

    /// Graceful shutdown: persist state and stop mpv (PRD section 14).
    pub async fn shutdown(&mut self) {
        self.capture_resume_point();
        self.maybe_save_session(self.state.playback.position_seconds, true);
        self.record_current(PlaybackOutcome::Stopped);
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

fn merge_playlist_tracks(target: &mut Playlist, incoming: Playlist) -> usize {
    let mut ids = target
        .tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let before = target.tracks.len();
    target.tracks.extend(
        incoming
            .tracks
            .into_iter()
            .filter(|track| ids.insert(track.id.clone())),
    );
    if target.description.is_empty() && !incoming.description.is_empty() {
        target.description = incoming.description;
    }
    let added = target.tracks.len() - before;
    if added > 0 {
        target.updated_at = chrono::Utc::now();
    }
    added
}

fn open_browser(url: &str) -> std::io::Result<()> {
    if !is_allowed_browser_url(url) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only YouTube HTTP(S) URLs are allowed",
        ));
    }
    browser_command(url).spawn().map(|_| ())
}

fn is_allowed_browser_url(url: &str) -> bool {
    let Some(remainder) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let host = authority.split(':').next().unwrap_or_default();
    matches!(
        host,
        "youtube.com" | "www.youtube.com" | "music.youtube.com" | "youtu.be"
    )
}

#[cfg(target_os = "macos")]
fn browser_command(url: &str) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    command.arg(url);
    command
}

#[cfg(target_os = "linux")]
fn browser_command(url: &str) -> std::process::Command {
    let mut command = std::process::Command::new("xdg-open");
    command.arg(url);
    command
}

#[cfg(target_os = "windows")]
fn browser_command(url: &str) -> std::process::Command {
    let mut command = std::process::Command::new("rundll32");
    command.args(["url.dll,FileProtocolHandler", url]);
    command
}

/// Whether an action may change text, order, or membership of a filterable list.
fn action_changes_filterable_data(action: &Action) -> bool {
    matches!(
        action,
        Action::AddToQueue(_)
            | Action::AddNext(_)
            | Action::RemoveSelectedFromQueue
            | Action::UndoQueueRemoval
            | Action::MoveSelectedInQueue(_)
            | Action::ClearQueueConfirmed
            | Action::PlayTrack(_)
            | Action::MixLoaded { .. }
            | Action::RadioTracksLoaded { .. }
            | Action::PlaylistSaved(_)
            | Action::RemoveSelectedFromPlaylist
            | Action::MoveSelectedInPlaylist(_)
            | Action::LoadPlaylistIntoQueue(_)
            | Action::AppendPlaylistToQueue(_)
            | Action::DeletePlaylistConfirmed(_)
            | Action::DeleteSelectedHistoryEntry
            | Action::ClearHistoryConfirmed
    )
}

#[cfg(test)]
mod command_path_tests {
    use super::*;

    fn test_app() -> (tempfile::TempDir, App) {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = AppPaths::with_data_dir(temp.path().to_path_buf());
        paths.ensure_dirs().expect("create test data root");
        let app = App::new(
            Config::default(),
            paths,
            AppState::new(),
            ratatui_image::picker::Picker::halfblocks(),
        );
        (temp, app)
    }

    #[tokio::test]
    async fn command_path_reduces_executes_and_persists_queue_action() {
        let (_temp, mut app) = test_app();
        let (action_tx, _action_rx) = mpsc::channel(4);
        app.handle_action(
            Action::AddToQueue(Track::new("id", "title", "artist")),
            &action_tx,
        )
        .await;

        assert_eq!(app.state.queue.tracks.len(), 1);
        let restored = crate::queue::service::load(&app.paths.queue_file()).expect("saved queue");
        assert_eq!(restored.tracks[0].id, "id");
    }

    #[tokio::test]
    async fn home_enter_dispatches_for_every_populated_iteration_four_section() {
        let (_temp, mut app) = test_app();
        app.state.view = View::Home;
        let resume = Track::new("resume", "Resume", "Artist");
        app.state.pending_resume = Some(crate::app::state::PendingResume {
            track: resume,
            position_seconds: 30.0,
            armed: true,
            play_on_load: false,
        });
        let (action_tx, mut action_rx) = mpsc::channel(8);
        app.state.home_section = crate::app::state::HomeSection::Resume;
        app.handle_action(Action::PlaySelected, &action_tx).await;
        assert!(matches!(action_rx.try_recv(), Ok(Action::PlayPause)));

        let mut history = HistoryService::load(&app.paths.history_file(), 500).expect("history");
        history.record(crate::history::model::HistoryEntry::from_track(
            &Track::new("recent", "Recent", "Artist"),
            None,
            PlaybackOutcome::Stopped,
            10,
        ));
        app.history = Some(history);
        app.state.home_section = crate::app::state::HomeSection::Recent;
        app.state.selected_index = 0;
        app.handle_action(Action::PlaySelected, &action_tx).await;
        assert!(
            matches!(action_rx.try_recv(), Ok(Action::PlayTrack(track)) if track.id == "recent")
        );

        let playlist = crate::playlists::Playlist::new("Home playlist");
        let playlist_id = playlist.id.clone();
        app.state.playlists.push(playlist);
        app.state.home_section = crate::app::state::HomeSection::Playlists;
        app.state.selected_index = 0;
        app.handle_action(Action::PlaySelected, &action_tx).await;
        assert!(matches!(
            action_rx.try_recv(),
            Ok(Action::LoadPlaylistIntoQueue(id)) if id == playlist_id
        ));
    }

    #[tokio::test]
    async fn playlist_mutations_emit_persistable_activity() {
        use crate::history::activity::ActivityKind;

        let (_temp, mut app) = test_app();
        let playlist = Playlist::new("Target");
        app.playlists.save(&playlist).expect("save target");
        app.state.playlists.push(playlist);
        app.state.picker = Some(crate::app::state::PickerState {
            track: Track::new("track", "Added track", "Artist"),
            filter: String::new(),
            selected: 0,
        });
        let (action_tx, _action_rx) = mpsc::channel(4);
        app.handle_action(Action::PickerSubmit, &action_tx).await;
        assert_eq!(
            app.state.activity.entries().front().map(|event| event.kind),
            Some(ActivityKind::AddedToPlaylist)
        );

        app.state.import = Some(ImportState::Review {
            summary: crate::playlists::import::ImportSummary {
                remote_title: "Imported".to_string(),
                remote_url: "https://www.youtube.com/playlist?list=safe".to_string(),
                total_entries: 0,
                imported: 0,
                deleted: 0,
                private: 0,
                unavailable: 0,
                duplicates: 0,
                missing_id: 0,
                missing_title: 0,
            },
            playlist: Box::new(Playlist::new("Imported")),
        });
        app.handle_action(Action::ConfirmImport, &action_tx).await;
        assert_eq!(
            app.state.activity.entries().front().map(|event| event.kind),
            Some(ActivityKind::PlaylistImported)
        );
    }

    #[tokio::test]
    async fn pasted_json_import_persists_every_playlist_after_full_validation() {
        let (_temp, mut app) = test_app();
        app.state.prompt = Some(crate::app::state::PromptState {
            purpose: PromptPurpose::ImportPlaylistJson,
            buffer: r#"{
              "version": 1,
              "playlists": [
                {"name":"One","tracks":[{"title":"A","channel":"Channel A","url":"https://youtu.be/aaaaaaaaaaa"}]},
                {"name":"Two","tracks":[{"title":"B","channel":"Channel B","url":"https://www.youtube.com/watch?v=bbbbbbbbbbb"}]}
              ]
            }"#
            .to_string(),
        });
        let (action_tx, _action_rx) = mpsc::channel(4);

        app.handle_action(Action::PromptSubmit, &action_tx).await;

        assert_eq!(app.playlists.list().expect("stored playlists").len(), 2);
        assert_eq!(app.state.playlists.len(), 2);
        assert!(app.state.prompt.is_none());
    }

    #[tokio::test]
    async fn repeated_json_import_merges_by_name_and_video_id() {
        let (_temp, mut app) = test_app();
        let (action_tx, _action_rx) = mpsc::channel(4);
        let first = r#"{
          "version":1,
          "playlists":[{"name":"Same Name","tracks":[
            {"title":"Original","channel":"A","url":"https://youtu.be/aaaaaaaaaaa"}
          ]}]
        }"#;
        let second = r#"{
          "version":1,
          "playlists":[{"name":" same name ","tracks":[
            {"title":"Duplicate metadata","channel":"B","url":"https://youtu.be/aaaaaaaaaaa"},
            {"title":"New track","channel":"C","url":"https://youtu.be/bbbbbbbbbbb"}
          ]}]
        }"#;

        for json in [first, second] {
            app.state.prompt = Some(crate::app::state::PromptState {
                purpose: PromptPurpose::ImportPlaylistJson,
                buffer: json.to_string(),
            });
            app.handle_action(Action::PromptSubmit, &action_tx).await;
        }

        let stored = app.playlists.list().expect("stored playlists");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].tracks.len(), 2);
        assert_eq!(stored[0].tracks[0].title, "Original");
        assert_eq!(app.state.playlists.len(), 1);
    }

    #[tokio::test]
    async fn playlist_editor_submit_persists_name_and_description() {
        let (_temp, mut app) = test_app();
        let playlist = Playlist::new("Before");
        let id = playlist.id.clone();
        app.playlists.save(&playlist).expect("seed playlist");
        app.state.playlists.push(playlist);
        app.state.selected_playlist = Some(0);
        app.state.playlist_editor = Some(crate::app::state::PlaylistEditorState {
            name: " After ".to_string(),
            description: " Clear context ".to_string(),
            field: crate::app::state::PlaylistEditorField::Description,
        });
        let (action_tx, _action_rx) = mpsc::channel(2);

        app.handle_action(Action::PlaylistEditorSubmit, &action_tx)
            .await;

        let stored = app.playlists.get(&id).expect("stored playlist");
        assert_eq!(stored.name, "After");
        assert_eq!(stored.description, "Clear context");
        assert!(app.state.playlist_editor.is_none());
    }

    #[tokio::test]
    async fn invalid_pasted_json_remains_editable() {
        let (_temp, mut app) = test_app();
        app.state.prompt = Some(crate::app::state::PromptState {
            purpose: PromptPurpose::ImportPlaylistJson,
            buffer: "not json".to_string(),
        });
        let (action_tx, _action_rx) = mpsc::channel(4);

        app.handle_action(Action::PromptSubmit, &action_tx).await;

        assert_eq!(
            app.state
                .prompt
                .as_ref()
                .map(|prompt| prompt.buffer.as_str()),
            Some("not json")
        );
        assert!(
            app.state
                .notification
                .as_ref()
                .is_some_and(|item| item.is_error)
        );
    }

    #[test]
    fn app_capture_resume_point_honors_boundaries() {
        let (_temp, mut app) = test_app();
        app.state.current_track = Some(Track::new("track", "Track", "Artist"));
        app.state.playback.duration_seconds = Some(100.0);

        app.state.playback.position_seconds = 10.0;
        app.capture_resume_point();
        app.state.playback.position_seconds = 95.0;
        app.capture_resume_point();
        assert_eq!(app.state.resume_points.len(), 0);

        app.state.playback.position_seconds = 30.0;
        app.capture_resume_point();
        assert_eq!(app.state.resume_points.len(), 1);
    }

    #[tokio::test]
    async fn pause_stop_and_track_change_capture_resume_points() {
        let (_temp, mut app) = test_app();
        let (action_tx, _action_rx) = mpsc::channel(2);
        let set_track = |app: &mut App, id: &str| {
            app.state.current_track = Some(Track::new(id, id, "Artist"));
            app.state.playback.position_seconds = 30.0;
            app.state.playback.duration_seconds = Some(100.0);
        };

        set_track(&mut app, "pause");
        app.on_playback_event(&PlaybackEvent::PauseChanged(true));
        set_track(&mut app, "stop");
        app.handle_action(Action::Stop, &action_tx).await;
        set_track(&mut app, "change");
        app.handle_action(
            Action::PlayTrack(Track::new("next", "Next", "Artist")),
            &action_tx,
        )
        .await;

        let ids = app
            .state
            .resume_points
            .entries()
            .iter()
            .map(|point| point.video_id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"pause"));
        assert!(ids.contains(&"stop"));
        assert!(ids.contains(&"change"));
    }

    #[tokio::test]
    async fn session_panels_restore_even_when_playback_resume_is_offline() {
        use crate::history::activity::{ActivityEvent, ActivityKind};

        let (_temp, mut app) = test_app();
        let mut document = crate::persistence::session::SessionDocument::new(None, 0.0, 50);
        document.activity.push(ActivityEvent::new(
            ActivityKind::Queued,
            "Persisted event",
            "detail",
        ));
        assert!(
            document
                .resume_points
                .record("persisted-track", 30.0, 100.0, chrono::Utc::now())
        );
        crate::persistence::session::save(&app.paths.session_file(), &document)
            .expect("save session");
        app.playback = None;
        let (action_tx, _action_rx) = mpsc::channel(2);

        app.init_session(&action_tx).await;

        assert_eq!(app.state.activity.len(), 1);
        assert_eq!(app.state.resume_points.len(), 1);
        assert!(app.state.pending_resume.is_none());
    }

    #[tokio::test]
    async fn search_queue_action_updates_selected_panel_membership_source() {
        let (_temp, mut app) = test_app();
        let track = Track::new("selected", "Selected", "Channel");
        app.state.view = View::Search;
        app.state.search = crate::media::search::SearchState::Results {
            query: "selected".to_string(),
            tracks: vec![track],
        };
        let (action_tx, mut action_rx) = mpsc::channel(4);
        app.handle_action(Action::AddSelectedToQueue, &action_tx)
            .await;
        let dispatched = action_rx.try_recv().expect("resolved queue action");
        assert!(matches!(&dispatched, Action::AddToQueue(track) if track.id == "selected"));
        app.handle_action(dispatched, &action_tx).await;
        assert!(
            app.state
                .queue
                .tracks
                .iter()
                .any(|track| track.id == "selected")
        );
    }

    #[test]
    fn unchanged_filter_reuses_derived_indices_until_list_mutation() {
        let (_temp, mut app) = test_app();
        app.state.view = View::Queue;
        app.state.queue.push(Track::new("a", "Alpha", "Artist"));
        app.state.queue.push(Track::new("b", "Beta", "Artist"));
        app.state.list_filter = Some("alpha".to_string());
        app.sync_list_view();
        let first = app
            .state
            .visible_indices
            .as_ref()
            .expect("filtered indices")
            .as_ptr();
        app.sync_list_view();
        assert_eq!(
            first,
            app.state
                .visible_indices
                .as_ref()
                .expect("cached indices")
                .as_ptr()
        );

        app.list_revision += 1;
        app.sync_list_view();
        assert_eq!(app.state.visible_indices.as_deref(), Some([0].as_slice()));
    }

    #[test]
    fn recent_history_selection_maps_through_newest_unique_entries() {
        let (_temp, mut app) = test_app();
        let history = app.history.as_mut().expect("history service");
        for track in [
            Track::new("a", "Old A", "Channel"),
            Track::new("b", "Only B", "Channel"),
            Track::new("a", "Newest A", "Channel"),
        ] {
            history.record(crate::history::model::HistoryEntry::from_track(
                &track,
                None,
                crate::history::model::PlaybackOutcome::Stopped,
                1,
            ));
        }
        app.state.view = View::History;
        app.sync_list_view();
        assert_eq!(app.state.history_len, 2);
        assert_eq!(
            app.state.visible_indices.as_deref(),
            Some([0, 1].as_slice())
        );

        app.state.list_filter = Some("Only B".to_string());
        app.sync_list_view();

        assert_eq!(app.state.visible_indices.as_deref(), Some([1].as_slice()));
        assert_eq!(
            app.resolve_selected_track().map(|track| track.id),
            Some("b".to_string())
        );
    }

    #[test]
    fn browser_dispatch_rejects_unsafe_schemes_and_hosts() {
        assert!(is_allowed_browser_url(
            "https://www.youtube.com/watch?v=safe"
        ));
        assert!(is_allowed_browser_url("https://youtu.be/safe"));
        assert!(!is_allowed_browser_url("file:///etc/passwd"));
        assert!(!is_allowed_browser_url(
            "https://youtube.com@example.com/attack"
        ));
        assert!(!is_allowed_browser_url("https://example.com/watch?v=no"));
    }
}
