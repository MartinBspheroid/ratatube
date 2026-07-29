//! Domain-specific application actions exposed through the [`Action`] facade.

mod history;
mod navigation;
mod playback;
mod playlists;
mod queue;
mod ui;

pub use history::HistoryAction;
pub use navigation::{ExternalCommandKind, ExternalCommandTarget, NavigationAction};
pub use playback::PlaybackAction;
pub use playlists::PlaylistAction;
pub use queue::QueueAction;
pub use ui::UiMsg;

#[cfg(test)]
mod tests {
    use super::{HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction};
    use crate::app::action::Action;

    #[test]
    fn facade_wraps_one_action_from_each_domain() {
        let actions = [
            Action::Ui(super::UiMsg::NextView),
            Action::Navigation(NavigationAction::OpenInBrowser),
            Action::Playback(PlaybackAction::PlayPause),
            Action::Queue(QueueAction::ClearQueue),
            Action::Playlists(PlaylistAction::OpenPlaylistDetail),
            Action::History(HistoryAction::ClearHistory),
        ];

        assert_eq!(actions.len(), 6);
    }
}
