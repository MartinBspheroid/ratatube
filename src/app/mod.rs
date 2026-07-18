//! Application orchestration: event loop wiring input, timers, mpv events,
//! and yt-dlp tasks into actions and effects (PRD sections 13 and 15).

pub mod action;
pub mod filter;
pub mod reducer;
pub mod state;

use std::time::{Duration, Instant};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::app::action::Action;
use crate::app::reducer::{Effect, reduce};
use crate::app::state::{AppState, Focus, ImportState, PromptPurpose, View};
use crate::config::Config;
use crate::error::Result;
use crate::history::{HistoryService, model::HistoryEntry, model::PlaybackOutcome};
use crate::input::keymap;
use crate::media::Track;
use crate::media::import::{InputKind, classify_input};
use crate::media::yt_dlp::YtDlp;
use crate::persistence::AppPaths;
use crate::playback::{MpvProcess, PlaybackController, PlaybackEvent};
use crate::playlists::Playlist;
use crate::playlists::service::PlaylistService;

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
    /// When the current track started, for history listened-duration.
    track_started_at: Option<Instant>,
    /// Last left-click timestamp for double-click detection.
    last_click: Option<Instant>,
    /// Terminal graphics picker (Kitty on Ghostty, halfblocks fallback).
    picker: ratatui_image::picker::Picker,
    /// Throttle for session snapshot writes during playback.
    last_session_save: Option<Instant>,
    /// CLI-provided startup behavior (--resume / play <query>).
    startup_intent: Option<StartupIntent>,
    /// Auto-play the first result of the next completed search.
    autoplay_first_search: bool,
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
            track_started_at: None,
            last_click: None,
            picker,
            last_session_save: None,
            startup_intent: None,
            autoplay_first_search: false,
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
        let socket = self.paths.data_dir.join("mpv.sock");
        let process = MpvProcess::spawn(
            &self.config.paths.mpv,
            &socket,
            self.config.playback.default_volume,
        )?;
        process.wait_for_socket(Duration::from_secs(5)).await?;
        let ipc = crate::playback::ipc::MpvIpc::connect(&socket, event_tx).await?;
        let mut controller = PlaybackController::new(ipc);
        controller.observe_defaults().await?;
        self.mpv = Some(process);
        self.playback = Some(controller);
        self.state.mpv_ready = true;
        Ok(())
    }

    /// Main event loop: terminal input, mpv events, render tick.
    pub async fn run(&mut self, terminal: &mut crate::ui::Terminal) -> Result<()> {
        let (action_tx, mut action_rx) = mpsc::channel::<Action>(256);
        let (playback_tx, mut playback_rx) = mpsc::channel::<PlaybackEvent>(256);

        if self.config.paths.mpv != "false"
            && let Err(err) = self.start_playback(playback_tx).await
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
                        _ => {}
                    }
                }
                Some(event) = playback_rx.recv() => {
                    tracing::debug!(?event, "mpv event");
                    self.on_playback_event(&event);
                    let _ = action_tx.send(Action::PlaybackEvent(event)).await;
                }
                Some(action) = action_rx.recv() => {
                    self.handle_action(action, &action_tx).await;
                }
                _ = tick.tick() => {
                    self.state.tick_spinner();
                    // Dismiss transient notifications after a few ticks.
                    if self.state.notification.is_some() && self.state.spinner_frame.is_multiple_of(20) {
                        self.state.notification = None;
                    }
                }
            }
            // Refresh filtered indices and the mirrored history length so
            // selection movement matches what is rendered.
            self.sync_list_view();
            crate::ui::render_with(terminal, &mut self.state, self.history.as_ref())?;
        }
        Ok(())
    }

    /// Restore the previous session (armed resume) and apply any CLI
    /// startup intent. Called once, right after mpv startup.
    async fn init_session(&mut self, action_tx: &mpsc::Sender<Action>) {
        use crate::config::ResumeMode;

        let mode = match self.startup_intent {
            Some(StartupIntent::Resume) => ResumeMode::Playing,
            _ => self.config.playback.resume_on_launch,
        };

        if let Some(StartupIntent::PlayQuery(query)) = self.startup_intent.clone() {
            self.state.view = View::Search;
            self.autoplay_first_search = true;
            self.submit_text_query(query, action_tx).await;
            return;
        }

        if mode == ResumeMode::Off || self.playback.is_none() {
            return;
        }
        let Some(doc) = crate::persistence::session::load(&self.paths.session_file()) else {
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

        // Resolve the stream in the background; the UI stays responsive and
        // shows the resume card as "resolving".
        let yt_dlp = self.yt_dlp.clone();
        let tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match yt_dlp.resolve_stream(&track.webpage_url).await {
                Ok(url) => Action::SessionStreamResolved {
                    track_id: track.id.clone(),
                    url,
                },
                Err(err) => Action::SessionResolveFailed {
                    track_id: track.id.clone(),
                    message: err.to_string(),
                },
            };
            let _ = tx.send(action).await;
        });
    }

    /// Persist the session snapshot, throttled to one write per interval.
    fn maybe_save_session(&mut self, position_seconds: f64, force: bool) {
        const SESSION_SAVE_INTERVAL: Duration = Duration::from_secs(5);
        if self.state.current_track.is_none() {
            return;
        }
        let due = self
            .last_session_save
            .is_none_or(|t| t.elapsed() >= SESSION_SAVE_INTERVAL);
        if !force && !due {
            return;
        }
        self.last_session_save = Some(Instant::now());
        let doc = crate::persistence::session::SessionDocument::new(
            self.state.current_track.clone(),
            position_seconds,
            self.state.playback.volume,
        );
        if let Err(err) = crate::persistence::session::save(&self.paths.session_file(), &doc) {
            tracing::warn!(?err, "session save failed");
        }
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
                    for (view, start, end) in crate::ui::widgets::tab_hit_zones(&icons) {
                        if mouse.column >= start && mouse.column < end {
                            let _ = action_tx.send(Action::Navigate(view)).await;
                            return;
                        }
                    }
                    return;
                }
                // Now-playing panel: click the progress row to seek. The bar
                // is suppressed on the Playing view (it has its own gauge).
                if self.state.has_now_playing() && self.state.view != View::NowPlaying {
                    let layout =
                        crate::ui::layout::AppLayout::new(self.state.screen_area, true, true);
                    let bar = layout.now_playing;
                    let gauge_row = bar.y + bar.height.saturating_sub(2);
                    if bar.height >= 3
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
                let area = self.state.main_area;
                // Inside the list region (below block border and, for the
                // search view, below the 3-row input box).
                let top = if self.state.view == View::Search {
                    area.y + 4
                } else {
                    area.y + 1
                };
                if mouse.row >= top && mouse.row < area.y + area.height.saturating_sub(1) {
                    let offset = if self.state.view == View::Search {
                        self.state.table_state.offset()
                    } else {
                        self.state.list_state.offset()
                    };
                    let index = offset + usize::from(mouse.row - top);
                    if index < self.state.active_list_len() {
                        self.state.selected_index = index;
                        let now = Instant::now();
                        let double_click = self
                            .last_click
                            .is_some_and(|t| now.duration_since(t) < Duration::from_millis(400));
                        self.last_click = Some(now);
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
        tracing::debug!(?action, "action");
        // Record skips before the reducer replaces the current track.
        match &action {
            Action::NextTrack | Action::PreviousTrack => {
                self.record_current(PlaybackOutcome::Skipped);
            }
            Action::ThumbnailLoaded { track_id, bytes } => {
                self.on_thumbnail_loaded(track_id.clone(), bytes.clone());
            }
            _ => {}
        }
        let effects = reduce(&mut self.state, action.clone());
        self.execute(effects, action_tx).await;
        self.handle_service_action(action, action_tx).await;
    }

    /// Route keyboard input: modals first, then focus-based keymap.
    async fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        // Import review modal: Enter confirms, Esc cancels.
        if matches!(self.state.import, Some(ImportState::Review { .. })) {
            let action = match key.code {
                KeyCode::Enter => Some(Action::ConfirmImport),
                KeyCode::Esc => Some(Action::CancelImport),
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
                    let query = std::mem::take(&mut self.state.search_input);
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
                        if let Some(filter) = &mut self.state.list_filter {
                            if filter.pop().is_none() {
                                self.state.list_filter = None;
                                self.state.focus = Focus::Content;
                            }
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
            HistoryViewMode::Recent => self.history.as_ref().map_or(0, |h| h.entries().len()),
            HistoryViewMode::Top => self.history.as_ref().map_or(0, |h| h.aggregate().len()),
        };

        let filter = self
            .state
            .list_filter
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if filter.is_empty() {
            self.state.visible_indices = None;
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
            rows.iter().map(|(text, outcome)| (text.clone(), outcome.as_deref())),
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
                let _ = action_tx.send(Action::SubmitSearch(url)).await;
            }
            InputKind::Playlist(_) => {
                let _ = action_tx.send(Action::StartImport(query)).await;
                self.state.focus = Focus::Content;
            }
            InputKind::Mix(_) => {
                self.state
                    .notify("Mix/radio URLs are not supported yet", false);
                self.state.focus = Focus::Content;
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
                Effect::RunImport { url } => {
                    self.spawn_import(url, action_tx.clone());
                }
                Effect::ResolveAndPlay {
                    track_index_in_queue,
                } => {
                    self.resolve_and_play(track_index_in_queue, action_tx).await;
                }
                Effect::SeekBy(seconds) => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.seek_by(seconds).await;
                    }
                }
                Effect::SeekTo(seconds) => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.seek_to(seconds).await;
                    }
                }
                Effect::TogglePause => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.toggle_pause().await;
                    }
                }
                Effect::AdjustVolume(delta) => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.adjust_volume(delta).await;
                    }
                }
                Effect::ToggleMute => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.toggle_mute().await;
                    }
                }
                Effect::StopPlayback => {
                    self.record_current(PlaybackOutcome::Stopped);
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.stop().await;
                    }
                }
                Effect::QuitMpv => {
                    if let Some(p) = self.playback.as_mut() {
                        let _ = p.quit().await;
                    }
                }
                Effect::PersistQueue => {
                    if let Err(err) =
                        crate::queue::service::save(&self.paths.queue_file(), &self.state.queue)
                    {
                        tracing::warn!(?err, "queue persistence failed");
                    }
                }
                Effect::PersistPlaylists | Effect::Exit => {}
            }
        }
    }

    /// Actions that need services (playlist storage, history) or full state.
    async fn handle_service_action(&mut self, action: Action, action_tx: &mpsc::Sender<Action>) {
        match action {
            Action::SessionStreamResolved { track_id, url } => {
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
                    Some(p) => p.load_at(&url, &title, Some(position), paused).await,
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
                        self.spawn_thumbnail_fetch(&track, action_tx);
                    }
                    Err(err) => {
                        self.state.pending_resume = None;
                        self.state.notify(&format!("Resume failed: {err}"), true);
                    }
                }
            }
            Action::SessionResolveFailed { track_id, message } => {
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
                    self.state
                        .notify("Couldn't resume the last session", true);
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
                    if let Err(err) = history.save() {
                        tracing::warn!(?err, "history save failed");
                    }
                    self.state.history_len = history.entries().len();
                    self.state.clamp_selection();
                }
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
            Action::MoveSelectedInPlaylist(delta) => {
                if self.state.view != View::PlaylistDetail {
                    return;
                }
                if self.state.visible_indices.is_some() {
                    self.state.notify("Clear the filter (Esc) to reorder", false);
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
                        (_, true) => {}
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
                    }
                }
            }
            Action::ConfirmImport => {
                if let Some(ImportState::Review { playlist, .. }) = self.state.import.take() {
                    match self.playlists.save(&playlist) {
                        Ok(()) => {
                            let saved = *playlist;
                            self.state
                                .notify(&format!("Imported \"{}\"", saved.name), false);
                            let _ = action_tx.send(Action::PlaylistSaved(saved)).await;
                        }
                        Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
                    }
                }
            }
            Action::ClearHistory => {
                if let Some(history) = self.history.as_mut() {
                    history.clear();
                    if let Err(err) = history.save() {
                        tracing::warn!(?err, "history save failed");
                    }
                }
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
                _ => None,
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
            self.state
                .notify(&format!("Already in \"{name}\""), false);
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
                    .notify(&format!("Added to \"{}\"", snapshot.name), false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
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
                self.state.notify("Playlist created", false);
            }
            Err(err) => self.state.notify(&format!("Save failed: {err}"), true),
        }
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
    }

    /// Fire an async yt-dlp search; completion arrives as an action.
    fn spawn_search(&self, query: String, generation: u64, action_tx: mpsc::Sender<Action>) {
        let yt_dlp = self.yt_dlp.clone();
        let limit = self.config.search.result_limit;
        tokio::spawn(async move {
            let action = match yt_dlp.search(&query, limit).await {
                Ok(tracks) => Action::SearchCompleted { generation, tracks },
                Err(err) => Action::SearchFailed {
                    generation,
                    message: err.to_string(),
                },
            };
            let _ = action_tx.send(action).await;
        });
    }

    /// Fire an async playlist import; completion arrives as an action.
    fn spawn_import(&self, url: String, action_tx: mpsc::Sender<Action>) {
        let yt_dlp = self.yt_dlp.clone();
        tokio::spawn(async move {
            let action = match yt_dlp.fetch_playlist(&url).await {
                Ok(fetch) => Action::ImportCompleted {
                    url,
                    title: fetch.title,
                    remote_id: fetch.remote_id,
                    tracks: fetch.tracks,
                    skipped: fetch.skipped,
                },
                Err(err) => Action::ImportFailed {
                    url,
                    message: err.to_string(),
                },
            };
            let _ = action_tx.send(action).await;
        });
    }

    /// Track playback lifecycle for history recording (PRD 10.11) and the
    /// resume-session snapshot.
    fn on_playback_event(&mut self, event: &PlaybackEvent) {
        match event {
            PlaybackEvent::Started => {
                self.track_started_at = Some(Instant::now());
                self.maybe_save_session(0.0, true);
            }
            PlaybackEvent::PositionChanged(position) => {
                self.maybe_save_session(*position, false);
            }
            PlaybackEvent::EndFile { reason } => {
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
        let (Some(track), Some(started)) =
            (self.state.current_track.clone(), self.track_started_at)
        else {
            return;
        };
        self.track_started_at = None;
        let Some(history) = self.history.as_mut() else {
            return;
        };
        let listened = started.elapsed().as_secs();
        history.record(HistoryEntry::from_track(&track, None, outcome, listened));
        if let Err(err) = history.save() {
            tracing::warn!(?err, "history save failed");
        }
    }

    /// Resolve the track at a queue position and start playback (PRD 10.4).
    ///
    /// On resolution failure, retries once; on continued failure and with
    /// `continueOnError`, advances through the queue until a track plays or
    /// the queue is exhausted (PRD 10.4, 16).
    async fn resolve_and_play(
        &mut self,
        mut queue_position: usize,
        action_tx: &mpsc::Sender<Action>,
    ) {
        loop {
            let Some(&track_index) = self.state.queue.order.get(queue_position) else {
                return;
            };
            let track = self.state.queue.tracks[track_index].clone();
            self.state.current_track = Some(track.clone());
            self.state
                .notify(&format!("Resolving: {}", track.title), false);

            let stream = self.yt_dlp.resolve_stream(&track.webpage_url).await;
            let url = match stream {
                Ok(url) => url,
                Err(first_err) => {
                    tracing::warn!(?first_err, track = %track.id, "resolve failed; retrying once");
                    match self.yt_dlp.resolve_stream(&track.webpage_url).await {
                        Ok(url) => url,
                        Err(err) => {
                            self.state
                                .notify(&format!("Unavailable: {}", track.title), true);
                            tracing::warn!(?err, track = %track.id, "retry failed");
                            if !self.config.playback.continue_on_error {
                                return;
                            }
                            // Advance to the next queue item and try again.
                            let effects = reduce(&mut self.state, Action::NextTrack);
                            for effect in &effects {
                                if matches!(effect, Effect::PersistQueue) {
                                    let _ = crate::queue::service::save(
                                        &self.paths.queue_file(),
                                        &self.state.queue,
                                    );
                                }
                            }
                            match self.state.queue.position {
                                Some(next) => {
                                    queue_position = next;
                                    continue;
                                }
                                None => return,
                            }
                        }
                    }
                }
            };
            if let Some(p) = self.playback.as_mut() {
                if let Err(err) = p.load(&url, &track.title).await {
                    self.state.notify(&format!("mpv: {err}"), true);
                } else {
                    self.spawn_details_fetch(&track, action_tx);
                    self.spawn_thumbnail_fetch(&track, action_tx);
                }
            } else {
                self.state.notify("mpv not running", true);
            }
            return;
        }
    }

    /// Fetch extended metadata in the background for the now-playing view.
    fn spawn_details_fetch(&self, track: &Track, action_tx: &mpsc::Sender<Action>) {
        let yt_dlp = self.yt_dlp.clone();
        let url = track.webpage_url.clone();
        let track_id = track.id.clone();
        let tx = action_tx.clone();
        tokio::spawn(async move {
            if let Ok(details) = yt_dlp.fetch_details(&url).await {
                let _ = tx
                    .send(Action::DetailsLoaded {
                        track_id,
                        details: Box::new(details),
                    })
                    .await;
            }
        });
    }

    /// Fetch the track's YouTube thumbnail in the background.
    ///
    /// Uses the deterministic i.ytimg.com URL; downloaded via curl so no
    /// HTTP client dependency is needed. Failures are silent — the UI
    /// simply shows no image.
    fn spawn_thumbnail_fetch(&self, track: &Track, action_tx: &mpsc::Sender<Action>) {
        let track_id = track.id.clone();
        let tx = action_tx.clone();
        tokio::spawn(async move {
            let url = format!("https://i.ytimg.com/vi/{track_id}/hqdefault.jpg");
            let output = tokio::process::Command::new("curl")
                .args(["-sfL", "--max-time", "15", &url])
                .stdin(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .output()
                .await;
            if let Ok(out) = output
                && out.status.success()
                && !out.stdout.is_empty()
            {
                let _ = tx
                    .send(Action::ThumbnailLoaded {
                        track_id,
                        bytes: out.stdout,
                    })
                    .await;
            }
        });
    }

    /// Decode downloaded thumbnail bytes into a resize protocol.
    fn on_thumbnail_loaded(&mut self, track_id: String, bytes: Vec<u8>) {
        if self.state.current_track.as_ref().map(|t| t.id.as_str()) != Some(track_id.as_str()) {
            return;
        }
        match image::load_from_memory(&bytes) {
            Ok(dyn_img) => {
                self.state.thumbnail = Some(self.picker.new_resize_protocol(dyn_img));
            }
            Err(err) => tracing::warn!(?err, "thumbnail decode failed"),
        }
    }

    /// Graceful shutdown: persist state and stop mpv (PRD section 14).
    pub async fn shutdown(&mut self) {
        self.maybe_save_session(self.state.playback.position_seconds, true);
        self.record_current(PlaybackOutcome::Stopped);
        if let Err(err) = crate::queue::service::save(&self.paths.queue_file(), &self.state.queue) {
            tracing::warn!(?err, "queue save on shutdown failed");
        }
        if let Some(history) = self.history.as_ref()
            && let Err(err) = history.save()
        {
            tracing::warn!(?err, "history save on shutdown failed");
        }
        if let Some(mut process) = self.mpv.take() {
            process.shutdown().await;
        }
    }
}
