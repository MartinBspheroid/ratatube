//! Application action facade for the event-driven architecture in PRD section 13.

pub use crate::app::actions::{
    ExternalCommandKind, ExternalCommandTarget, HistoryAction, NavigationAction, PlaybackAction,
    PlaylistAction, QueueAction, UiMsg,
};

/// A state-changing intent produced by input, timers, or subprocess events and
/// grouped by the responsibility that handles it.
#[derive(Debug, Clone)]
pub enum Action {
    /// Client-local presentation transitions; never routed to the daemon.
    Ui(UiMsg),
    /// Navigation, search, selection, and context-menu intent.
    Navigation(NavigationAction),
    /// Playback and media-lifecycle intent.
    Playback(PlaybackAction),
    /// Queue mutation or queue-resolution intent.
    Queue(QueueAction),
    /// Playlist, import, and playlist-modal intent.
    Playlists(PlaylistAction),
    /// History, activity, or notification intent.
    History(HistoryAction),
}

/// Payload-free identity used to distinguish action domains and inner variants in diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActionDiagnosticIdentity {
    Ui(std::mem::Discriminant<UiMsg>),
    Navigation(std::mem::Discriminant<NavigationAction>),
    Playback(std::mem::Discriminant<PlaybackAction>),
    Queue(std::mem::Discriminant<QueueAction>),
    Playlists(std::mem::Discriminant<PlaylistAction>),
    History(std::mem::Discriminant<HistoryAction>),
}

impl Action {
    /// Return the domain plus inner discriminant without formatting action payloads.
    pub(super) fn diagnostic_identity(&self) -> ActionDiagnosticIdentity {
        match self {
            Self::Ui(msg) => ActionDiagnosticIdentity::Ui(std::mem::discriminant(msg)),
            Self::Navigation(action) => {
                ActionDiagnosticIdentity::Navigation(std::mem::discriminant(action))
            }
            Self::Playback(action) => {
                ActionDiagnosticIdentity::Playback(std::mem::discriminant(action))
            }
            Self::Queue(action) => ActionDiagnosticIdentity::Queue(std::mem::discriminant(action)),
            Self::Playlists(action) => {
                ActionDiagnosticIdentity::Playlists(std::mem::discriminant(action))
            }
            Self::History(action) => {
                ActionDiagnosticIdentity::History(std::mem::discriminant(action))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, NavigationAction, PlaybackAction, UiMsg};

    #[test]
    fn diagnostic_identity_distinguishes_variants_within_one_domain() {
        let navigate = Action::Ui(UiMsg::Navigate(crate::app::state::View::Home));
        let open_help = Action::Ui(UiMsg::OpenHelp);

        assert_ne!(
            navigate.diagnostic_identity(),
            open_help.diagnostic_identity()
        );
    }

    #[test]
    fn diagnostic_identity_distinguishes_actions_across_domains() {
        let navigation = Action::Navigation(NavigationAction::Quit);
        let playback = Action::Playback(PlaybackAction::Stop);

        assert_ne!(
            navigation.diagnostic_identity(),
            playback.diagnostic_identity()
        );
    }
}
