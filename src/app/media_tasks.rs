//! Stream resolution and extended-metadata background tasks.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaybackAction};
use crate::app::operations::OperationKind;
use crate::app::reducer::reduce;
use crate::media::Track;

impl App {
    /// Resolve the track at a queue position and start playback (PRD 10.4).
    ///
    /// Resolution retries once in an owned, cancellable task. A matching
    /// failure action advances the queue when `continue_on_error` is enabled.
    pub(super) fn spawn_resolve_and_play(
        &mut self,
        queue_position: usize,
        action_tx: &mpsc::Sender<Action>,
    ) {
        let Some(track) = self
            .state
            .domain
            .queue
            .order
            .get(queue_position)
            .and_then(|index| self.state.domain.queue.tracks.get(*index))
            .cloned()
        else {
            return;
        };
        let prefetched = self.prefetched.take().and_then(|(id, url, resolved_at)| {
            (id == track.id && resolved_at.elapsed() < Duration::from_secs(2 * 3600)).then_some(url)
        });
        let ticket = self.operations.start(OperationKind::Playback);
        let operation_id = ticket.id();
        let _ = reduce(
            &mut self.state,
            Action::Playback(PlaybackAction::PlaybackResolveStarted {
                operation_id,
                queue_position,
                track_id: track.id.clone(),
            }),
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
                    Ok(url) => Action::Playback(PlaybackAction::PlaybackResolved {
                        operation_id,
                        queue_position,
                        track_id: track.id,
                        url,
                    }),
                    Err(err) => Action::Playback(PlaybackAction::PlaybackResolveFailed {
                        operation_id,
                        queue_position,
                        track_id: track.id,
                        message: err.to_string(),
                    }),
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Playback, operation_id, handle);
    }

    /// Fetch extended metadata in the background for the now-playing view.
    pub(super) fn spawn_details_fetch(&mut self, track: &Track, action_tx: &mpsc::Sender<Action>) {
        let ticket = self.operations.start(OperationKind::Details);
        let operation_id = ticket.id();
        let yt_dlp = self.yt_dlp.clone();
        let url = track.webpage_url.clone();
        let track_id = track.id.clone();
        let _ = reduce(
            &mut self.state,
            Action::Playback(PlaybackAction::DetailsStarted {
                operation_id,
                track_id: track_id.clone(),
            }),
        );
        let tx = action_tx.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_details(&url) => match result {
                    Ok(details) => Action::Playback(PlaybackAction::DetailsLoaded {
                        operation_id,
                        track_id,
                        details: Box::new(details),
                    }),
                    Err(err) => Action::Playback(PlaybackAction::DetailsFailed {
                        operation_id,
                        track_id,
                        message: err.to_string(),
                    }),
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Details, operation_id, handle);
    }
}
