//! Queue actions that resolve selected content through application services.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaylistAction, QueueAction};

impl App {
    /// Resolve selected queue content through application-owned state and services.
    pub(super) async fn handle_queue_service(
        &mut self,
        action: QueueAction,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match action {
            QueueAction::AddSelectedToQueue => {
                if let Some(track) = self.resolve_selected_track() {
                    let _ = action_tx
                        .send(Action::Queue(QueueAction::AddToQueue(track)))
                        .await;
                }
            }
            QueueAction::AddSelectedAsNext => {
                if let Some(track) = self.resolve_selected_track() {
                    let _ = action_tx
                        .send(Action::Queue(QueueAction::AddNext(track)))
                        .await;
                }
            }
            QueueAction::LoadSelectedPlaylistIntoQueue => {
                if let Some(id) = self.selected_playlist_id() {
                    let _ = action_tx
                        .send(Action::Playlists(PlaylistAction::LoadPlaylistIntoQueue(id)))
                        .await;
                }
            }
            QueueAction::AppendSelectedPlaylistToQueue => {
                if let Some(id) = self.selected_playlist_id() {
                    let _ = action_tx
                        .send(Action::Playlists(PlaylistAction::AppendPlaylistToQueue(id)))
                        .await;
                }
            }
            // Explicit queue mutations and completions are fully reduced; only
            // the "selected" variants need the service layer to resolve them.
            QueueAction::AddToQueue(_)
            | QueueAction::AddNext(_)
            | QueueAction::RemoveSelectedFromQueue
            | QueueAction::RemoveTrackOccurrence { .. }
            | QueueAction::UndoQueueRemoval
            | QueueAction::MoveSelectedInQueue(_)
            | QueueAction::MoveTrack { .. }
            | QueueAction::ClearQueue
            | QueueAction::ClearQueueConfirmed
            | QueueAction::QueueLoaded(_)
            | QueueAction::QueueExhausted => {}
        }
    }
}
