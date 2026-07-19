//! Application-state facade organized by responsibility (PRD section 13).

mod app_state;
mod modals;
mod navigation;
mod notifications;
mod operations;
mod selection;

pub use app_state::AppState;
pub use modals::{
    ConfirmState, PickerState, PlaylistEditorField, PlaylistEditorState, PromptPurpose,
    PromptState, TrackContextMenuState, TrackDetailsModalState,
};
pub use navigation::{Focus, HistoryViewMode, HomeSection, PlayingPane, View};
pub use notifications::{Notification, SleepTimer};
pub use operations::{DetailsStatus, ImportState, OperationStatus, PendingResume};
