//! Service-backed universal track-context operations.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction, PlaybackAction, PlaylistAction, QueueAction};
use crate::app::browser::open_browser;
use crate::app::track_context::TrackContextAction;

impl App {
    /// Resolve the active track with optional history data and open its menu.
    pub(super) fn open_track_context(&mut self) {
        crate::app::track_context::open_track_context(&mut self.state, self.history.as_ref());
    }

    /// Execute or dispatch the stable operation selected in the open menu.
    pub(super) async fn submit_track_context(&mut self, action_tx: &mpsc::Sender<Action>) {
        let Some((action, track)) = self.state.track_context_menu.as_ref().and_then(|menu| {
            let action = menu.context.actions.get(menu.selected)?.clone();
            Some((action, menu.context.track.clone()))
        }) else {
            return;
        };
        match action {
            TrackContextAction::OpenInBrowser => match open_browser(&track.webpage_url) {
                Ok(()) => {
                    self.state.track_context_menu = None;
                    self.state.notify("Opened in browser", false);
                }
                Err(error) => self
                    .state
                    .notify(&format!("Couldn't open browser: {error}"), true),
            },
            TrackContextAction::CopyUrl => {
                match crate::platform::clipboard::copy_url(&track.webpage_url) {
                    Ok(()) => {
                        self.state.track_context_menu = None;
                        self.state.notify("Copied URL", false);
                    }
                    Err(error) => self
                        .state
                        .notify(&format!("Couldn't copy URL: {error}"), true),
                }
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
                        Action::Queue(QueueAction::RemoveTrackOccurrence {
                            order_index,
                            expected_track: track,
                        })
                    }
                    TrackContextAction::RemoveFromPlaylist {
                        playlist_id,
                        track_index,
                    } => Action::Playlists(PlaylistAction::RemoveTrackOccurrence {
                        playlist_id,
                        track_index,
                        expected_track: track,
                    }),
                    TrackContextAction::OpenInBrowser | TrackContextAction::CopyUrl => {
                        unreachable!("process operations handled above")
                    }
                };
                self.state.track_context_menu = None;
                let _ = action_tx.send(dispatched).await;
            }
        }
    }
}
