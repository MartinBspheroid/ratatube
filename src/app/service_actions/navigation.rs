//! Navigation actions that cross process or asynchronous service boundaries.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction, PlaybackAction};
use crate::app::browser::open_browser;
use crate::app::state::View;

impl App {
    /// Handle navigation actions that cross process or asynchronous boundaries.
    pub(super) async fn handle_navigation_service(
        &mut self,
        action: NavigationAction,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match action {
            NavigationAction::OpenTrackContext => self.open_track_context(),
            NavigationAction::SubmitTrackContext => self.submit_track_context(action_tx).await,
            NavigationAction::OpenInBrowser => {
                let track = match self.state.view {
                    View::Search => self.resolve_selected_track(),
                    View::NowPlaying => self.state.current_track.clone(),
                    _ => None,
                };
                match track {
                    Some(track) => match open_browser(&track.webpage_url) {
                        Ok(()) => self.state.notify("Opened in browser", false),
                        Err(error) => self
                            .state
                            .notify(&format!("Couldn't open browser: {error}"), true),
                    },
                    None => self.state.notify("No track selected", true),
                }
            }
            NavigationAction::SearchCompleted { .. } if self.autoplay_first_search => {
                self.autoplay_first_search = false;
                if self.state.active_list_len() > 0 {
                    self.state.selected_index = 0;
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::PlaySelected))
                        .await;
                }
            }
            _ => {}
        }
    }
}
