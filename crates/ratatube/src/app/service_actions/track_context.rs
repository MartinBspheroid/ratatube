//! Service-backed universal track-context operations.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{
    Action, ExternalCommandKind, ExternalCommandTarget, NavigationAction, PlaybackAction,
    PlaylistAction, QueueAction,
};
use crate::app::track_context::{CollectionRevision, TrackContextAction};

impl App {
    /// Resolve the active track with optional history data and open its menu.
    pub(super) fn open_track_context(&mut self) {
        crate::app::track_context::open_track_context(&mut self.state, self.history.as_deref());
    }

    /// Execute or dispatch the stable operation selected in the open menu.
    pub(super) async fn submit_track_context(&mut self, action_tx: &mpsc::Sender<Action>) {
        let Some((action, track, collection_revision)) =
            self.state.ui.track_context_menu.as_ref().and_then(|menu| {
                let action = menu.context.actions.get(menu.selected)?.clone();
                Some((
                    action,
                    menu.context.track.clone(),
                    menu.context.collection_revision,
                ))
            })
        else {
            return;
        };
        match action {
            TrackContextAction::OpenInBrowser => self.spawn_external_command(
                ExternalCommandKind::Browser,
                ExternalCommandTarget::TrackContext {
                    track_id: track.id,
                    generation: self.state.ui.track_context_generation,
                },
                track.webpage_url,
                action_tx.clone(),
            ),
            TrackContextAction::CopyUrl => {
                self.spawn_external_command(
                    ExternalCommandKind::Clipboard,
                    ExternalCommandTarget::TrackContext {
                        track_id: track.id,
                        generation: self.state.ui.track_context_generation,
                    },
                    track.webpage_url,
                    action_tx.clone(),
                );
            }
            action => {
                let dispatched = match action {
                    TrackContextAction::PlayNow => {
                        Action::Playback(PlaybackAction::PlayTrack(track))
                    }
                    TrackContextAction::PlayNext => Action::Queue(QueueAction::AddNext(track)),
                    TrackContextAction::AddToQueue => Action::Queue(QueueAction::AddToQueue(track)),
                    TrackContextAction::AddToPlaylist => {
                        Action::Playlists(PlaylistAction::OpenPlaylistPickerForTrack(track))
                    }
                    TrackContextAction::VisitChannel => {
                        Action::Navigation(NavigationAction::VisitChannel(track))
                    }
                    TrackContextAction::ShowDetails => {
                        Action::Navigation(NavigationAction::ShowTrackDetails(track))
                    }
                    TrackContextAction::RemoveFromQueue { order_index } => {
                        let Some(CollectionRevision::Queue(expected_revision)) =
                            collection_revision
                        else {
                            return;
                        };
                        Action::Queue(QueueAction::RemoveTrackOccurrence {
                            order_index,
                            expected_track: track,
                            expected_revision,
                        })
                    }
                    TrackContextAction::RemoveFromPlaylist {
                        playlist_id,
                        track_index,
                    } => {
                        let Some(CollectionRevision::Playlists(expected_revision)) =
                            collection_revision
                        else {
                            return;
                        };
                        Action::Playlists(PlaylistAction::RemoveTrackOccurrence {
                            playlist_id,
                            track_index,
                            expected_track: track,
                            expected_revision,
                        })
                    }
                    TrackContextAction::OpenInBrowser | TrackContextAction::CopyUrl => {
                        unreachable!("process operations handled above")
                    }
                };
                self.state.ui.track_context_menu = None;
                if matches!(
                    dispatched,
                    Action::Playlists(PlaylistAction::OpenPlaylistPickerForTrack(_))
                        | Action::Navigation(NavigationAction::ShowTrackDetails(_))
                ) {
                    match dispatched {
                        Action::Playlists(PlaylistAction::OpenPlaylistPickerForTrack(track)) => {
                            self.state.show_playlist_picker(track);
                        }
                        Action::Navigation(NavigationAction::ShowTrackDetails(track)) => {
                            self.state.show_track_details(track);
                        }
                        _ => unreachable!("modal transition checked above"),
                    }
                } else {
                    let _ = action_tx.send(dispatched).await;
                }
            }
        }
    }
}
