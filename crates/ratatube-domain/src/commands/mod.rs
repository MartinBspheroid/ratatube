//! Per-context command enums.
//!
//! Every enum here is one context's complete vocabulary. No type in this
//! module spans contexts, so no reducer or router can end in a world-spanning
//! catch-all — the failure mode that once dropped by-id playlist commands.

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
