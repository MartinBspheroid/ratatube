//! Top-level reducer dispatch, stale-operation rejection, and post-reduce work.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{
    Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction,
};
use crate::app::operations::OperationKind;
use crate::app::reducer::reduce;
use crate::history::model::PlaybackOutcome;

impl App {
    /// Reduce an action and execute resulting effects plus service work.
    pub(super) async fn handle_action(&mut self, action: Action, action_tx: &mpsc::Sender<Action>) {
        if action_changes_filterable_data(&action) {
            self.list_revision = self.list_revision.wrapping_add(1);
            self.filter_sync_key = None;
        }
        match &action {
            Action::Playback(PlaybackAction::PlaybackResolved { operation_id, .. })
            | Action::Playback(PlaybackAction::PlaybackResolveFailed { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::Playback, *operation_id)
                {
                    return;
                }
            }
            Action::Playlists(PlaylistAction::ImportCompleted { operation_id, .. })
            | Action::Playlists(PlaylistAction::ImportFailed { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::Import, *operation_id)
                {
                    return;
                }
            }
            Action::Playlists(PlaylistAction::CancelImport) => {
                self.operations.cancel(OperationKind::Import)
            }
            Action::Playback(PlaybackAction::RadioTracksLoaded { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::Radio, *operation_id)
                {
                    return;
                }
            }
            Action::Playback(PlaybackAction::DetailsLoaded { operation_id, .. })
            | Action::Playback(PlaybackAction::DetailsFailed { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::Details, *operation_id)
                {
                    return;
                }
            }
            Action::Playback(PlaybackAction::PrefetchResolved { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::Prefetch, *operation_id)
                {
                    return;
                }
            }
            Action::Playback(PlaybackAction::ThumbnailLoaded { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::Thumbnail, *operation_id)
                {
                    return;
                }
            }
            Action::Playback(PlaybackAction::SearchThumbnailLoaded { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::SearchThumbnail, *operation_id)
                {
                    return;
                }
            }
            Action::Playback(PlaybackAction::SessionStreamResolved { operation_id, .. })
            | Action::Playback(PlaybackAction::SessionResolveFailed { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::Session, *operation_id)
                {
                    return;
                }
            }
            Action::Playback(PlaybackAction::MixLoaded { operation_id, .. }) => {
                if !self.operations.complete(OperationKind::Mix, *operation_id) {
                    return;
                }
            }
            Action::Navigation(NavigationAction::ExternalCommandCompleted {
                operation_id, ..
            }) => {
                if !self
                    .operations
                    .complete(OperationKind::ExternalCommand, *operation_id)
                {
                    return;
                }
            }
            Action::Navigation(NavigationAction::ChannelResolved { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::ChannelResolve, *operation_id)
                {
                    return;
                }
            }
            Action::Navigation(NavigationAction::ChannelPageLoaded { operation_id, .. }) => {
                if !self
                    .operations
                    .complete(OperationKind::ChannelPage, *operation_id)
                {
                    return;
                }
            }
            Action::Playback(PlaybackAction::ToggleRadio) if self.state.domain.radio => {
                self.operations.cancel(OperationKind::Radio);
                self.radio_fetching = false;
            }
            _ => {}
        }
        tracing::debug!(action = ?action.diagnostic_identity(), "action");
        match &action {
            Action::Playback(PlaybackAction::NextTrack)
            | Action::Playback(PlaybackAction::PreviousTrack) => {
                // Record skips before the reducer replaces the current track.
                self.capture_resume_point();
                self.record_current(PlaybackOutcome::Skipped);
            }
            Action::Playback(PlaybackAction::Stop)
            | Action::Playback(PlaybackAction::PlayTrack(_))
            | Action::Playback(PlaybackAction::PlaybackResolved { .. }) => {
                self.capture_resume_point();
            }
            Action::Playback(PlaybackAction::ThumbnailLoaded {
                track_id, bytes, ..
            }) => self.on_thumbnail_loaded(track_id.clone(), bytes.clone()),
            Action::Playback(PlaybackAction::SearchThumbnailLoaded {
                track_id, bytes, ..
            }) => self.on_search_thumbnail_loaded(track_id.clone(), bytes.clone()),
            Action::Playback(PlaybackAction::SeekForward)
            | Action::Playback(PlaybackAction::SeekBackward)
            | Action::Playback(PlaybackAction::SeekForwardLarge)
            | Action::Playback(PlaybackAction::SeekBackwardLarge)
            | Action::Playback(PlaybackAction::SeekToFraction(_))
            | Action::Playback(PlaybackAction::NextChapter)
            | Action::Playback(PlaybackAction::PreviousChapter) => self.listening.seeking(),
            _ => {}
        }
        // The watermark spans reduce AND service handlers, so events cover
        // every domain mutation an action triggers on either path.
        let watermark = crate::app::domain_event::DomainWatermark::capture(&self.state.domain);
        let effects = reduce(&mut self.state, action.clone());
        self.execute(effects, action_tx).await;
        self.handle_service_action(action.clone(), action_tx).await;
        self.sync_search_thumbnail(action_tx);
        let events = watermark.events_since(&self.state.domain, &action);
        crate::app::ui_sync::apply_domain_events(&self.state.domain, &mut self.state.ui, &events);
    }
}

/// Whether an action may change text, order, or membership of a filterable list.
fn action_changes_filterable_data(action: &Action) -> bool {
    matches!(
        action,
        Action::Queue(QueueAction::AddToQueue(_))
            | Action::Queue(QueueAction::AddNext(_))
            | Action::Queue(QueueAction::RemoveSelectedFromQueue)
            | Action::Queue(QueueAction::RemoveTrackOccurrence { .. })
            | Action::Queue(QueueAction::UndoQueueRemoval)
            | Action::Queue(QueueAction::MoveSelectedInQueue(_))
            | Action::Queue(QueueAction::ClearQueueConfirmed)
            | Action::Playback(PlaybackAction::PlayTrack(_))
            | Action::Playback(PlaybackAction::MixLoaded { .. })
            | Action::Playback(PlaybackAction::RadioTracksLoaded { .. })
            | Action::Playlists(PlaylistAction::PlaylistSaved(_))
            | Action::Playlists(PlaylistAction::RemoveSelectedFromPlaylist)
            | Action::Playlists(PlaylistAction::RemoveTrackOccurrence { .. })
            | Action::Playlists(PlaylistAction::MoveSelectedInPlaylist(_))
            | Action::Playlists(PlaylistAction::LoadPlaylistIntoQueue(_))
            | Action::Playlists(PlaylistAction::AppendPlaylistToQueue(_))
            | Action::Playlists(PlaylistAction::DeletePlaylistConfirmed(_))
            | Action::History(HistoryAction::DeleteSelectedHistoryEntry)
            | Action::History(HistoryAction::ClearHistoryConfirmed)
    )
}
