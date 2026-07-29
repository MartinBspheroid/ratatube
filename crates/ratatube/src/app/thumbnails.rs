//! Thumbnail download, cancellation, decoding, and selection synchronization.

use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaybackAction};
use crate::app::operations::OperationKind;
use crate::app::state::View;
use crate::media::Track;

#[derive(Debug, Clone, Copy)]
pub(super) enum ThumbnailPurpose {
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

impl App {
    /// Fetch a YouTube thumbnail in a cancellable task.
    ///
    /// The deterministic `i.ytimg.com` URL is downloaded with curl to avoid an
    /// HTTP-client dependency. Failure is silent and leaves the UI without an image.
    pub(super) fn spawn_thumbnail_fetch(
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
                    ThumbnailPurpose::CurrentTrack => {
                        Action::Playback(PlaybackAction::ThumbnailLoaded {
                            operation_id,
                            track_id,
                            bytes,
                        })
                    }
                    ThumbnailPurpose::SearchSelection => {
                        Action::Playback(PlaybackAction::SearchThumbnailLoaded {
                            operation_id,
                            track_id,
                            bytes,
                        })
                    }
                };
                let _ = tx.send(action).await;
            }
        });
        self.operations.attach(operation_kind, operation_id, handle);
    }

    /// Decode current-track thumbnail bytes into a resize protocol only while
    /// that track is still active.
    pub(super) fn on_thumbnail_loaded(&mut self, track_id: String, bytes: Vec<u8>) {
        if self
            .state
            .domain
            .current_track
            .as_ref()
            .map(|track| track.id.as_str())
            != Some(track_id.as_str())
        {
            return;
        }
        match crate::media::decode_thumbnail(&bytes) {
            Ok(image) => {
                self.state.ui.thumbnail = self
                    .picker
                    .as_mut()
                    .map(|picker| picker.new_resize_protocol(image));
            }
            Err(err) => tracing::warn!(?err, "thumbnail decode failed"),
        }
    }

    /// Start or reuse the thumbnail preview for the selected Search result.
    pub(super) fn sync_search_thumbnail(&mut self, action_tx: &mpsc::Sender<Action>) {
        if self.state.ui.view != View::Search {
            return;
        }
        let selected = match &self.state.domain.search {
            crate::media::search::SearchState::Results { tracks, .. } => {
                tracks.get(self.state.ui.selected_index).cloned()
            }
            _ => None,
        };
        let Some(track) = selected else {
            self.state.ui.search_thumbnail_track_id = None;
            self.state.ui.search_thumbnail = None;
            self.operations.cancel(OperationKind::SearchThumbnail);
            return;
        };
        if self.state.ui.search_thumbnail_track_id.as_deref() == Some(track.id.as_str()) {
            return;
        }
        self.state.ui.search_thumbnail_track_id = Some(track.id.clone());
        self.state.ui.search_thumbnail = None;
        self.spawn_thumbnail_fetch(&track, ThumbnailPurpose::SearchSelection, action_tx);
    }

    /// Decode a selected-result thumbnail only if that result is still active.
    pub(super) fn on_search_thumbnail_loaded(&mut self, track_id: String, bytes: Vec<u8>) {
        if self.state.ui.search_thumbnail_track_id.as_deref() != Some(track_id.as_str()) {
            return;
        }
        match crate::media::decode_thumbnail(&bytes) {
            Ok(image) => {
                self.state.ui.search_thumbnail = self
                    .picker
                    .as_mut()
                    .map(|picker| picker.new_resize_protocol(image));
            }
            Err(err) => tracing::warn!(?err, "search thumbnail decode failed"),
        }
    }
}
