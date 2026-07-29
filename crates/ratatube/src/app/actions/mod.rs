//! Action vocabulary: domain commands from `ratatube-domain` plus the
//! client-local presentation messages that never cross the wire.

mod ui;

pub use ratatube_domain::commands::{
    ExternalCommandKind, ExternalCommandTarget, HistoryAction, NavigationAction, PlaybackAction,
    PlaylistAction, QueueAction,
};
pub use ui::UiMsg;
