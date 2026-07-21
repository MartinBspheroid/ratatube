//! Radio refill and next-track stream prefetch after playback starts.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaybackAction};
use crate::app::operations::OperationKind;
use crate::app::reducer::reduce;

impl App {
    /// After a track starts, prefetch the next stream and, in radio mode,
    /// refill the queue when playback reaches its boundary.
    pub(super) fn after_track_started(&mut self, action_tx: &mpsc::Sender<Action>) {
        let len = self.state.domain.queue.order.len();
        let Some(position) = self.state.domain.queue.position else {
            return;
        };

        if self.state.domain.radio
            && position + 1 >= len
            && !self.radio_fetching
            && let Some(track) = self.state.domain.current_track.clone()
        {
            self.spawn_radio_refill(track.id, action_tx);
        }

        if self.state.domain.queue.repeat == crate::queue::RepeatMode::Track {
            return;
        }
        // Prefetching is pointless when repeating the current track.
        let next_position = if position + 1 < len {
            Some(position + 1)
        } else if self.state.domain.queue.repeat == crate::queue::RepeatMode::Queue && len > 0 {
            Some(0)
        } else {
            None
        };
        let Some(next_track) = next_position
            .and_then(|queue_position| self.state.domain.queue.order.get(queue_position))
            .map(|&index| self.state.domain.queue.tracks[index].clone())
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
            let result = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.resolve_stream(&next_track.webpage_url) => result,
            };
            // Failures are silent because playback resolves on demand as a fallback.
            if let Ok(url) = result {
                let _ = tx
                    .send(Action::Playback(PlaybackAction::PrefetchResolved {
                        operation_id,
                        track_id: next_track.id,
                        url,
                    }))
                    .await;
            }
        });
        self.operations
            .attach(OperationKind::Prefetch, operation_id, handle);
    }

    /// Fetch more tracks from YouTube's mix for `seed_id` in radio mode.
    pub(super) fn spawn_radio_refill(&mut self, seed_id: String, action_tx: &mpsc::Sender<Action>) {
        self.radio_fetching = true;
        let ticket = self.operations.start(OperationKind::Radio);
        let operation_id = ticket.id();
        let _ = reduce(
            &mut self.state,
            Action::Playback(PlaybackAction::RadioRefillStarted { operation_id }),
        );
        let yt_dlp = self.yt_dlp.clone();
        let tx = action_tx.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_mix(&seed_id) => match result {
                    Ok(fetch) => Action::Playback(PlaybackAction::RadioTracksLoaded {
                        operation_id,
                        tracks: fetch.tracks,
                    }),
                    Err(err) => {
                        tracing::warn!(?err, "radio refill failed");
                        Action::Playback(PlaybackAction::RadioTracksLoaded {
                            operation_id,
                            tracks: Vec::new(),
                        })
                    }
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Radio, operation_id, handle);
    }
}
