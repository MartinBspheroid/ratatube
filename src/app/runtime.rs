//! Main event loop and runtime channel coordination.

use std::time::{Duration, Instant};

use crossterm::event::{Event, EventStream, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaybackAction};
use crate::app::operations::OperationKind;
use crate::error::Result;
use crate::playback::PlaybackEvent;

impl App {
    /// Main event loop: terminal input, mpv events, render tick.
    pub async fn run(&mut self, terminal: &mut crate::ui::Terminal) -> Result<()> {
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
                        Some(Ok(Event::Paste(text))) => {
                            self.handle_paste(text, &action_tx).await;
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
                    let _ = action_tx.send(Action::Playback(PlaybackAction::PlaybackEvent(event))).await;
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
                    // Sleep timer expiry owns the stop action and clears the timer once.
                    if let Some(timer) = self.state.sleep_timer
                        && timer.deadline <= Instant::now()
                    {
                        self.state.sleep_timer = None;
                        self.state.notify("Sleep timer: stopping playback", false);
                        let _ = action_tx.send(Action::Playback(PlaybackAction::Stop)).await;
                    }
                }
            }
            // Keep filter indices and mirrored history length synchronized with
            // the exact state rendered in this loop iteration.
            self.sync_list_view();
            crate::ui::render_with(terminal, &mut self.state, self.history.as_ref())?;
        }
        self.operations.shutdown(Duration::from_secs(1)).await;
        Ok(())
    }
}
