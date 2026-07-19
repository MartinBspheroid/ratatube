//! Service-backed action handling partitioned by action domain.

mod history;
mod navigation;
mod playback;
mod playlist_editing;
mod playlist_storage;
mod playlists;
mod queue;
mod track_context;

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::Action;

impl App {
    /// Handle work that needs services or full application state.
    pub(super) async fn handle_service_action(
        &mut self,
        action: Action,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match action {
            Action::Navigation(action) => self.handle_navigation_service(action, action_tx).await,
            Action::Playback(action) => self.handle_playback_service(action, action_tx).await,
            Action::Queue(action) => self.handle_queue_service(action, action_tx).await,
            Action::Playlists(action) => self.handle_playlist_service(action, action_tx).await,
            Action::History(action) => self.handle_history_service(action),
        }
    }
}
