//! Attached-mode TUI event loop: renders from `(UiState, DomainMirror)`,
//! routes actions per `client_route`, and reattaches on daemon loss.

use std::time::{Duration, Instant};

use crossterm::event::{Event, EventStream, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::Action;
use crate::app::client_route::Route;
use crate::app::domain_event::DomainEvent;
use crate::app::state::{Focus, PromptPurpose};
use crate::client::{CommandSender, Connection};
use crate::error::Result;
use crate::media::search::SearchState;
use crate::protocol::{Command, DaemonFrame, ReplyBody, ReplyResult, WireEvent};

/// An in-flight search request: reply id and the query it carried.
type PendingSearch = Option<(u64, String)>;

impl App {
    /// Attached-mode event loop: terminal input, daemon frames, render tick.
    pub async fn run_client(
        &mut self,
        terminal: &mut crate::ui::Terminal,
        connection: Connection,
    ) -> Result<()> {
        let (action_tx, mut action_rx) = mpsc::channel::<Action>(256);
        let (mut commands, snapshot, mut frames) = connection.into_stream();
        crate::client::mirror::apply_snapshot(&mut self.state.domain, snapshot);
        let mut pending_search: PendingSearch = None;
        // Keeps a parked replacement channel alive after a failed reattach so
        // the select arm blocks instead of spinning on a closed receiver.
        let mut parked_keepalive: Option<mpsc::Sender<DaemonFrame>> = None;

        let mut events = EventStream::new();
        let mut tick = tokio::time::interval(Duration::from_millis(
            self.config.ui.progress_refresh_ms.max(100),
        ));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        while self.state.ui.running {
            tokio::select! {
                maybe_event = events.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            self.handle_key(key, &action_tx).await;
                        }
                        Some(Ok(Event::Mouse(mouse))) => {
                            self.handle_mouse(mouse, &action_tx).await;
                        }
                        Some(Ok(Event::Paste(text))) => {
                            self.handle_paste(text, &action_tx).await;
                        }
                        _ => {}
                    }
                }
                frame = frames.recv() => {
                    match frame {
                        Some(frame) => self.on_daemon_frame(frame, &mut pending_search),
                        None => match self.reattach().await {
                            Some((sender, receiver)) => {
                                commands = sender;
                                frames = receiver;
                                parked_keepalive = None;
                            }
                            None => {
                                self.state
                                    .notify("Daemon connection lost; press q to quit", true);
                                let (keepalive, parked) = mpsc::channel(1);
                                parked_keepalive = Some(keepalive);
                                frames = parked;
                            }
                        },
                    }
                }
                Some(action) = action_rx.recv() => {
                    self.handle_client_action(action, &mut commands, &mut pending_search, &action_tx)
                        .await;
                }
                _ = tick.tick() => {
                    self.state.tick_spinner();
                    if self.state.ui.notification.as_ref().is_some_and(|notification| {
                        notification.is_expired_at(Instant::now())
                    }) {
                        self.state.ui.notification = None;
                    }
                }
            }
            self.sync_list_view();
            crate::ui::render_with(terminal, &mut self.state, self.history.as_ref())?;
        }
        drop(parked_keepalive);
        self.operations.shutdown(Duration::from_secs(1)).await;
        Ok(())
    }

    /// Bounded respawn-and-reattach after connection loss (mpv policy).
    async fn reattach(&mut self) -> Option<(CommandSender, mpsc::Receiver<DaemonFrame>)> {
        self.state
            .notify("Daemon disconnected; reconnecting…", true);
        for _ in 0..3 {
            match crate::client::connect_or_spawn(&self.paths).await {
                Ok(connection) => {
                    let (sender, snapshot, receiver) = connection.into_stream();
                    crate::client::mirror::apply_snapshot(&mut self.state.domain, snapshot);
                    self.state.notify("Reconnected to daemon", false);
                    return Some((sender, receiver));
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        }
        None
    }

    /// Apply one daemon frame to the mirror and the UI.
    fn on_daemon_frame(&mut self, frame: DaemonFrame, pending_search: &mut PendingSearch) {
        match frame {
            DaemonFrame::Event { event } => {
                let ui_event = domain_event_for(&event);
                let history_changed = matches!(*event, WireEvent::HistoryChanged);
                let lists_changed = matches!(
                    *event,
                    WireEvent::QueueChanged { .. } | WireEvent::PlaylistsChanged { .. }
                );
                crate::client::mirror::apply_event(&mut self.state.domain, *event);
                if let Some(ui_event) = ui_event {
                    crate::app::ui_sync::apply_domain_events(
                        &self.state.domain,
                        &mut self.state.ui,
                        &[ui_event],
                    );
                }
                if history_changed {
                    self.reload_history();
                }
                if lists_changed || history_changed {
                    self.list_revision = self.list_revision.wrapping_add(1);
                    self.filter_sync_key = None;
                }
            }
            DaemonFrame::Reply { id, result } => self.on_reply(id, result, pending_search),
            DaemonFrame::Welcome { .. } => {}
        }
    }

    fn on_reply(&mut self, id: u64, result: ReplyResult, pending_search: &mut PendingSearch) {
        let for_search = pending_search
            .as_ref()
            .is_some_and(|(pending_id, _)| *pending_id == id);
        match result {
            ReplyResult::Result(ReplyBody::Tracks { tracks }) if for_search => {
                let (_, query) = pending_search.take().expect("pending search");
                if tracks.is_empty() {
                    self.state.notify("No results", false);
                }
                self.state.domain.search = SearchState::Results { query, tracks };
                self.state.ui.selected_index = 0;
            }
            ReplyResult::Error(message) => {
                if for_search {
                    let (_, query) = pending_search.take().expect("pending search");
                    self.state.domain.search = SearchState::Failed {
                        query,
                        message: message.clone(),
                    };
                }
                self.state.notify(&message, true);
            }
            ReplyResult::Result(_) => {}
        }
    }

    /// Route one action per the client routing table.
    async fn handle_client_action(
        &mut self,
        action: Action,
        commands: &mut CommandSender,
        pending_search: &mut PendingSearch,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match crate::app::client_route::route(&action, &self.state, self.history.as_ref()) {
            Route::Local => {
                let _ = self.handle_action(action, action_tx).await;
            }
            Route::Quit => self.state.ui.running = false,
            Route::Send(command) => {
                if commands.send(command).await.is_err() {
                    self.state.notify("Daemon is not reachable", true);
                }
            }
            Route::Search { query, exact } => {
                self.state.domain.search_generation += 1;
                self.state.domain.search = SearchState::Searching {
                    query: query.clone(),
                    generation: self.state.domain.search_generation,
                };
                self.state.ui.focus = Focus::Content;
                let command = if exact {
                    Command::SearchExact { url: query.clone() }
                } else {
                    Command::Search {
                        query: query.clone(),
                    }
                };
                match commands.send(command).await {
                    Ok(id) => *pending_search = Some((id, query)),
                    Err(_) => self.state.notify("Daemon is not reachable", true),
                }
            }
            Route::PromptSubmit => self.client_prompt_submit(commands).await,
            Route::PickerSubmit => self.client_picker_submit(commands).await,
            Route::EditorSubmit => self.client_editor_submit(commands).await,
            Route::Deferred(message) => self.state.notify(message, false),
            Route::Ignore => {}
        }
    }

    /// Resolve the picker candidate against the mirror and send the add.
    async fn client_picker_submit(&mut self, commands: &mut CommandSender) {
        let Some(picker) = self.state.ui.picker.take() else {
            return;
        };
        let (create_new, matching) =
            crate::app::filter::picker_candidates(&self.state.domain.playlists, &picker.filter);
        let command = if create_new && picker.selected == 0 {
            Command::PlaylistAddTrackNew {
                name: picker.filter.trim().to_string(),
                track: picker.track,
            }
        } else {
            let offset = usize::from(create_new);
            let Some(&index) = picker
                .selected
                .checked_sub(offset)
                .and_then(|position| matching.get(position))
            else {
                return;
            };
            let Some(playlist) = self.state.domain.playlists.get(index) else {
                return;
            };
            // Immediate feedback from the mirror; the daemon re-checks.
            if playlist.tracks.iter().any(|t| t.id == picker.track.id) {
                let name = playlist.name.clone();
                self.state.notify(&format!("Already in \"{name}\""), false);
                return;
            }
            Command::PlaylistAddTrack {
                playlist_id: playlist.id.clone(),
                track: picker.track,
            }
        };
        if commands.send(command).await.is_err() {
            self.state.notify("Daemon is not reachable", true);
        }
    }

    /// Resolve the selected playlist id and send the metadata edit.
    async fn client_editor_submit(&mut self, commands: &mut CommandSender) {
        let Some(editor) = self.state.ui.playlist_editor.take() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            self.state.ui.playlist_editor = Some(editor);
            self.state.notify("Playlist name is required", true);
            return;
        }
        let Some(id) = self
            .state
            .ui
            .selected_playlist
            .and_then(|index| self.state.domain.playlists.get(index))
            .map(|playlist| playlist.id.clone())
        else {
            return;
        };
        let description = editor.description.trim().to_string();
        if commands
            .send(Command::PlaylistEdit {
                id,
                name,
                description,
            })
            .await
            .is_err()
        {
            self.state.notify("Daemon is not reachable", true);
        }
    }

    /// Purpose-dependent prompt submission in attached mode.
    async fn client_prompt_submit(&mut self, commands: &mut CommandSender) {
        let Some(prompt) = self.state.ui.prompt.take() else {
            return;
        };
        let text = prompt.buffer.trim().to_string();
        if text.is_empty() {
            let required = match prompt.purpose {
                PromptPurpose::ImportPlaylistUrl => "A URL is required",
                PromptPurpose::ImportPlaylistJson => "Playlist JSON is required",
                _ => "A playlist name is required",
            };
            self.state.notify(required, true);
            return;
        }
        let command = match prompt.purpose {
            PromptPurpose::SaveQueueAsPlaylist => Command::SaveQueueAsPlaylist { name: text },
            PromptPurpose::NewPlaylist => Command::PlaylistCreate { name: text },
            PromptPurpose::RenamePlaylist => {
                let index = self.state.resolve_index(self.state.ui.selected_index);
                match self
                    .state
                    .domain
                    .playlists
                    .get(index)
                    .map(|playlist| playlist.id.clone())
                {
                    Some(id) => Command::PlaylistRename { id, name: text },
                    None => return,
                }
            }
            PromptPurpose::ImportPlaylistUrl | PromptPurpose::ImportPlaylistJson => {
                self.state
                    .notify("Playlist import is not yet available while attached", false);
                return;
            }
        };
        if commands.send(command).await.is_err() {
            self.state.notify("Daemon is not reachable", true);
        }
    }

    /// Reload the read-only history mirror after a `HistoryChanged` event.
    fn reload_history(&mut self) {
        if !self.config.history.enabled {
            return;
        }
        self.history = crate::history::HistoryService::load(
            &self.paths.history_file(),
            self.config.history.max_entries,
        )
        .map_err(|err| tracing::warn!(?err, "history reload failed"))
        .ok();
    }
}

/// Map a wire event onto the `DomainEvent` used for UI invariants.
fn domain_event_for(event: &WireEvent) -> Option<DomainEvent> {
    match event {
        WireEvent::QueueChanged { .. } => Some(DomainEvent::QueueChanged),
        WireEvent::PlaybackProgress { .. } => Some(DomainEvent::PlaybackChanged),
        WireEvent::TrackChanged { .. } => Some(DomainEvent::TrackChanged),
        WireEvent::TrackDetailsChanged { .. } => Some(DomainEvent::TrackDetailsChanged),
        WireEvent::PlaylistsChanged { .. } => Some(DomainEvent::PlaylistsChanged),
        WireEvent::HistoryChanged => Some(DomainEvent::HistoryChanged),
        WireEvent::ImportChanged => Some(DomainEvent::ImportChanged),
        WireEvent::Health { .. } => Some(DomainEvent::Health),
    }
}
