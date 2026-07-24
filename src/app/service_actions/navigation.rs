//! Navigation actions that cross process or asynchronous service boundaries.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{
    Action, ExternalCommandKind, ExternalCommandTarget, NavigationAction, PlaybackAction,
};
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
            NavigationAction::OpenSettings => self.open_settings(),
            NavigationAction::SubmitTrackContext => self.submit_track_context(action_tx).await,
            NavigationAction::OpenInBrowser => {
                let track = match self.state.ui.view {
                    View::Search => self.resolve_selected_track(),
                    View::NowPlaying => self.state.domain.current_track.clone(),
                    _ => None,
                };
                match track {
                    Some(track) => self.spawn_external_command(
                        ExternalCommandKind::Browser,
                        ExternalCommandTarget::Direct,
                        track.webpage_url,
                        action_tx.clone(),
                    ),
                    None => self.state.notify("No track selected", true),
                }
            }
            NavigationAction::ExternalCommandCompleted {
                command,
                target,
                result,
                ..
            } => self.finish_external_command(command, target, result),
            NavigationAction::VisitChannel(track) => {
                self.visit_channel(track, action_tx.clone());
            }
            NavigationAction::ChannelResolved { result, .. } => {
                self.finish_channel_resolve(result, action_tx.clone());
            }
            NavigationAction::ChannelPageLoaded {
                channel_url,
                page,
                result,
                ..
            } => self.finish_channel_page(&channel_url, page, result),
            NavigationAction::LoadMoreChannel | NavigationAction::RetryChannel => {
                if let Some(page) = self
                    .state
                    .domain
                    .channel
                    .as_ref()
                    .map(|channel| channel.next_page)
                {
                    self.spawn_channel_page(page, action_tx.clone());
                }
            }
            NavigationAction::BackFromChannel => self.leave_channel(),
            NavigationAction::SearchCompleted { .. } if self.autoplay_first_search => {
                self.autoplay_first_search = false;
                if self.state.active_list_len() > 0 {
                    self.state.ui.selected_index = 0;
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::PlaySelected))
                        .await;
                }
            }
            _ => {}
        }
    }

    /// Open the settings menu seeded with current configuration values;
    /// only the service layer sees `Config`, so the reducer cannot do this.
    fn open_settings(&mut self) {
        let theme = self.state.ui.theme;
        self.state.ui.settings = Some(crate::app::state::SettingsState {
            tab: crate::app::state::SettingsTab::Appearance,
            selected: crate::app::reducer::family_index(theme.family()),
            original_theme: theme,
            icons: self.config.ui.icons,
            resume: self.config.playback.resume_on_launch,
        });
    }
}
