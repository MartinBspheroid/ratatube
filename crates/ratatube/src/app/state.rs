//! Application-state facade organized by responsibility (PRD section 13).
//!
//! The domain half lives in `ratatube-domain`; the UI half is local. Both are
//! re-exported here so call sites keep one state vocabulary.

mod app_state;
mod modals;
mod notifications;
mod selection;
mod ui_state;

pub use app_state::AppState;
pub(crate) use modals::ModalCapture;
pub use modals::{
    ConfirmState, PickerState, PlaylistEditorField, PlaylistEditorState, PromptState,
    SETTINGS_GENERAL_ROWS, SettingsState, SettingsTab, TrackContextMenuState,
    TrackDetailsModalState,
};
pub use notifications::Notification;
pub use ratatube_domain::state::{
    ChannelNavigationSnapshot, ChannelState, DetailsStatus, DomainState, Focus, HistoryViewMode,
    HomeSection, ImportState, OperationStatus, PendingResume, PlayingPane, PromptPurpose,
    SleepTimer, View,
};
pub use ui_state::{HomeHitZone, UiState};
