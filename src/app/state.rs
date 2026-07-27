//! Application-state facade organized by responsibility (PRD section 13).

mod app_state;
mod domain_state;
mod modals;
mod navigation;
mod notifications;
mod operations;
mod selection;
mod ui_state;

pub use crate::app::channel::{ChannelNavigationSnapshot, ChannelState};
pub use app_state::AppState;
pub use domain_state::DomainState;
pub(crate) use modals::ModalCapture;
pub use modals::{
    ConfirmState, PickerState, PlaylistEditorField, PlaylistEditorState, PromptPurpose,
    PromptState, SETTINGS_GENERAL_ROWS, SettingsState, SettingsTab, TrackContextMenuState,
    TrackDetailsModalState,
};
pub use navigation::{Focus, HistoryViewMode, HomeSection, PlayingPane, View};
pub use notifications::{Notification, SleepTimer};
pub use operations::{DetailsStatus, ImportState, OperationStatus, PendingResume};
pub use ui_state::{HomeHitZone, UiState};
