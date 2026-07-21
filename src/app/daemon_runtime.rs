//! Headless daemon event loop: the single-process runtime minus terminal
//! input and rendering, plus the socket server front-end.

use std::time::{Duration, Instant};

use tokio::net::UnixListener;
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction, PlaybackAction, QueueAction};
use crate::app::domain_event::DomainEvent;
use crate::app::operations::OperationKind;
use crate::app::state::DomainState;
use crate::daemon::server::{Clients, ServerMessage, spawn_accept_loop};
use crate::error::Result;
use crate::playback::PlaybackEvent;
use crate::protocol::{
    Command, DaemonFrame, PROTOCOL_VERSION, ReplyBody, ReplyResult, Snapshot, WireEvent,
};

/// A search awaiting results on behalf of one client request.
struct PendingSearch {
    client_id: u64,
    reply_id: u64,
    generation: u64,
}

impl App {
    /// Daemon event loop: socket commands, mpv events, timers. No terminal.
    pub async fn run_daemon(&mut self, listener: UnixListener) -> Result<()> {
        let (action_tx, mut action_rx) = mpsc::channel::<Action>(256);
        let (playback_tx, mut playback_rx) = mpsc::channel::<PlaybackEvent>(256);
        let (recovery_tx, mut recovery_rx) = mpsc::channel::<(
            crate::app::operations::OperationId,
            crate::app::PlaybackRecoveryResult,
        )>(1);
        let (persistence_writer, mut persistence_rx) =
            crate::persistence::writer::PersistenceWriter::new();
        self.persistence_writer = Some(persistence_writer);

        if self.config.paths.mpv != "false"
            && let Err(err) = self.start_playback(playback_tx.clone()).await
        {
            tracing::warn!(?err, "mpv unavailable at daemon start");
        }
        self.init_session(&action_tx).await;

        let (server_tx, mut server_rx) = mpsc::channel::<ServerMessage>(256);
        spawn_accept_loop(listener, server_tx);
        let mut clients = Clients::default();
        let mut pending_search: Option<PendingSearch> = None;

        let mut tick = tokio::time::interval(Duration::from_millis(1000));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        while self.state.ui.running {
            tokio::select! {
                Some(message) = server_rx.recv() => {
                    self.handle_server_message(
                        message,
                        &mut clients,
                        &mut pending_search,
                        &action_tx,
                    )
                    .await;
                }
                Some(event) = playback_rx.recv() => {
                    if let Some(playback) = self.playback.as_mut() {
                        playback.on_event(&event);
                    }
                    self.on_playback_event(&event);
                    if event == PlaybackEvent::Shutdown && self.state.ui.running {
                        self.begin_playback_recovery(playback_tx.clone(), recovery_tx.clone());
                    }
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::PlaybackEvent(event)))
                        .await;
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
                                self.state.domain.mpv_ready = true;
                                clients.broadcast(&health_frame(&self.state.domain));
                            }
                            Err(err) => tracing::warn!(?err, "mpv restart failed"),
                        }
                    }
                }
                Some(outcome) = persistence_rx.recv() => {
                    if let Err(err) = outcome.result {
                        tracing::warn!(?err, key = %outcome.key, "persistence failed");
                    }
                }
                Some(action) = action_rx.recv() => {
                    self.settle_pending_search(&action, &mut pending_search, &mut clients);
                    let events = self.handle_action(action, &action_tx).await;
                    broadcast_events(&self.state.domain, &events, &mut clients);
                }
                _ = tick.tick() => {
                    if let Some(timer) = self.state.domain.sleep_timer
                        && timer.deadline <= Instant::now()
                    {
                        self.state.domain.sleep_timer = None;
                        let _ = action_tx.send(Action::Playback(PlaybackAction::Stop)).await;
                    }
                }
            }
        }
        self.operations.shutdown(Duration::from_secs(1)).await;
        Ok(())
    }

    /// Register clients, translate commands to actions, and reply.
    async fn handle_server_message(
        &mut self,
        message: ServerMessage,
        clients: &mut Clients,
        pending_search: &mut Option<PendingSearch>,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match message {
            ServerMessage::Connected { client_id, sender } => {
                clients.insert(client_id, sender);
                clients.send_to(
                    client_id,
                    DaemonFrame::Welcome {
                        protocol: PROTOCOL_VERSION,
                        snapshot: Box::new(Snapshot::from(&self.state.domain)),
                    },
                );
            }
            ServerMessage::Disconnected { client_id } => clients.remove(client_id),
            ServerMessage::Command {
                client_id,
                id,
                command,
            } => {
                let (result, events) = self
                    .apply_command(client_id, id, command, pending_search, action_tx)
                    .await;
                if let Some(result) = result {
                    clients.send_to(client_id, DaemonFrame::Reply { id, result });
                }
                broadcast_events(&self.state.domain, &events, clients);
            }
        }
    }

    /// Translate one wire command into actions. A `None` reply is deferred
    /// (search answers arrive with the completion action); the event list is
    /// broadcast by the caller.
    async fn apply_command(
        &mut self,
        client_id: u64,
        reply_id: u64,
        command: Command,
        pending_search: &mut Option<PendingSearch>,
        action_tx: &mpsc::Sender<Action>,
    ) -> (Option<ReplyResult>, Vec<DomainEvent>) {
        let ack = Some(ReplyResult::Result(ReplyBody::Ack));
        let action = match command {
            Command::Status => {
                return (
                    Some(ReplyResult::Result(ReplyBody::Status {
                        snapshot: Box::new(Snapshot::from(&self.state.domain)),
                    })),
                    Vec::new(),
                );
            }
            Command::Shutdown => Action::Navigation(NavigationAction::Quit),
            Command::PlayPause => Action::Playback(PlaybackAction::PlayPause),
            Command::Stop => Action::Playback(PlaybackAction::Stop),
            Command::Next => Action::Playback(PlaybackAction::NextTrack),
            Command::Previous => Action::Playback(PlaybackAction::PreviousTrack),
            Command::Seek { seconds } => Action::Playback(PlaybackAction::SeekBy(seconds)),
            Command::Volume { delta } => Action::Playback(PlaybackAction::VolumeBy(delta)),
            Command::ToggleShuffle => Action::Playback(PlaybackAction::ToggleShuffle),
            Command::CycleRepeat => Action::Playback(PlaybackAction::CycleRepeat),
            Command::PlayTrack { track } => Action::Playback(PlaybackAction::PlayTrack(track)),
            Command::PlayQuery { query } => {
                self.autoplay_first_search = true;
                Action::Navigation(NavigationAction::SubmitSearch(query))
            }
            Command::QueueAdd { track, next } => {
                if next {
                    Action::Queue(QueueAction::AddNext(track))
                } else {
                    Action::Queue(QueueAction::AddToQueue(track))
                }
            }
            Command::QueueRemove {
                order_index,
                expected_revision,
            } => {
                let Some(expected_track) = self
                    .state
                    .domain
                    .queue
                    .order
                    .get(order_index)
                    .and_then(|&index| self.state.domain.queue.tracks.get(index))
                    .cloned()
                else {
                    return (
                        Some(ReplyResult::Error(format!(
                            "no queue entry at position {order_index}"
                        ))),
                        Vec::new(),
                    );
                };
                Action::Queue(QueueAction::RemoveTrackOccurrence {
                    order_index,
                    expected_track,
                    expected_revision,
                })
            }
            Command::QueueMove { from, to } => Action::Queue(QueueAction::MoveTrack { from, to }),
            Command::QueueClear => Action::Queue(QueueAction::ClearQueueConfirmed),
            Command::QueueUndo => Action::Queue(QueueAction::UndoQueueRemoval),
            Command::Search { query } => {
                let action = Action::Navigation(NavigationAction::SubmitSearch(query));
                let events = self.handle_action(action, action_tx).await;
                // SubmitSearch bumps the generation; remember whose reply it is.
                *pending_search = Some(PendingSearch {
                    client_id,
                    reply_id,
                    generation: self.state.domain.search_generation,
                });
                return (None, events);
            }
        };
        let events = self.handle_action(action, action_tx).await;
        (ack, events)
    }

    /// Answer a pending search reply when its completion action arrives.
    fn settle_pending_search(
        &self,
        action: &Action,
        pending_search: &mut Option<PendingSearch>,
        clients: &mut Clients,
    ) {
        let Some(pending) = pending_search.as_ref() else {
            return;
        };
        match action {
            Action::Navigation(NavigationAction::SearchCompleted { generation, tracks })
                if *generation == pending.generation =>
            {
                clients.send_to(
                    pending.client_id,
                    DaemonFrame::Reply {
                        id: pending.reply_id,
                        result: ReplyResult::Result(ReplyBody::Tracks {
                            tracks: tracks.clone(),
                        }),
                    },
                );
                *pending_search = None;
            }
            Action::Navigation(NavigationAction::SearchFailed {
                generation,
                message,
            }) if *generation == pending.generation => {
                clients.send_to(
                    pending.client_id,
                    DaemonFrame::Reply {
                        id: pending.reply_id,
                        result: ReplyResult::Error(message.clone()),
                    },
                );
                *pending_search = None;
            }
            _ => {}
        }
        // Superseded searches (a newer generation) never settle; drop them.
        if let Some(pending) = pending_search.as_ref()
            && pending.generation < self.state.domain.search_generation
        {
            *pending_search = None;
        }
    }
}

/// Map domain events to wire events with fresh payloads and broadcast them.
fn broadcast_events(domain: &DomainState, events: &[DomainEvent], clients: &mut Clients) {
    for event in events {
        if let Some(wire) = wire_event(domain, *event) {
            clients.broadcast(&DaemonFrame::Event {
                event: Box::new(wire),
            });
        }
    }
}

fn wire_event(domain: &DomainState, event: DomainEvent) -> Option<WireEvent> {
    match event {
        DomainEvent::QueueChanged => Some(WireEvent::QueueChanged {
            queue: domain.queue.clone(),
            queue_revision: domain.queue_revision,
        }),
        DomainEvent::PlaybackChanged => Some(WireEvent::PlaybackProgress {
            playback: domain.playback.clone(),
        }),
        DomainEvent::TrackChanged => Some(WireEvent::TrackChanged {
            track: domain.current_track.clone(),
        }),
        DomainEvent::TrackDetailsChanged => Some(WireEvent::TrackDetailsChanged {
            details: domain.current_details.clone(),
        }),
        DomainEvent::PlaylistsChanged => Some(WireEvent::PlaylistsChanged {
            playlists: domain.playlists.clone(),
        }),
        DomainEvent::HistoryChanged => Some(WireEvent::HistoryChanged),
        DomainEvent::ImportChanged => Some(WireEvent::ImportChanged),
        DomainEvent::Health => Some(health_event(domain)),
        // Search and channel data are per-client request/reply concerns.
        DomainEvent::SearchChanged | DomainEvent::ChannelChanged => None,
    }
}

fn health_event(domain: &DomainState) -> WireEvent {
    WireEvent::Health {
        health: crate::protocol::Health {
            mpv_ready: domain.mpv_ready,
            yt_dlp_ready: domain.yt_dlp_ready,
        },
    }
}

fn health_frame(domain: &DomainState) -> DaemonFrame {
    DaemonFrame::Event {
        event: Box::new(health_event(domain)),
    }
}
