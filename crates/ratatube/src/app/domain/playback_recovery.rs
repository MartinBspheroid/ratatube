//! Bounded mpv restart and reconnection orchestration.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::app::operations::OperationKind;
use crate::app::{App, PlaybackRecoveryResult};
use crate::playback::PlaybackEvent;

impl App {
    /// Start one bounded mpv restart attempt and register it for cancellation and stale rejection.
    pub(in crate::app) fn begin_playback_recovery(
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
                    match App::connect_playback(
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
}
