//! Client-local presentation messages that never cross the daemon wire.
//!
//! `UiMsg` deliberately implements no serde traits: a variant added here is
//! unroutable to the daemon by construction, so the client router needs no
//! per-variant decision for it (workspace-split design, phase 1).

use crate::state::View;

/// A presentation-only transition: views, help, settings, selection, panes,
/// and notification chrome. Reduced locally in every mode.
#[derive(Debug, Clone)]
pub enum UiMsg {
    Navigate(View),
    /// Open help while remembering the current view.
    OpenHelp,
    /// Return from help to the view that opened it.
    CloseHelp,
    /// Scroll the help document by signed rows.
    ScrollHelp(i32),
    NextView,
    PreviousView,
    /// Move Home section focus forward/backward.
    CycleHomeSection(i32),
    /// Toggle the selected-result detail overlay on narrow Search layouts.
    ToggleSearchDetail,
    /// Open the ctrl+p settings menu seeded from current configuration.
    OpenSettings,
    /// Close the settings menu, restoring the theme it opened with.
    CloseSettings,
    /// Move to the next settings tab.
    SettingsCycleTab,
    /// Move the settings selection by signed rows; previews themes live.
    SettingsMove(i32),
    /// Cycle the selected settings value by a signed step.
    SettingsAdjust(i32),
    /// Persist every settings draft and close the menu.
    SettingsSubmit,
    SelectNext,
    SelectPrevious,
    /// Scroll the now-playing description panel.
    ScrollNowPlaying(i32),
    /// Toggle the Playing view's right pane between chapters and description.
    ToggleNowPlayingPane,
    /// Switch between info and queue focus in the ultra-wide Playing view.
    CyclePlayingPane,
    ToggleHistoryViewMode,
    DismissNotification,
    ToggleNotificationLog,
}
