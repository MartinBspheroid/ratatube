//! UI-half reducers: transitions that touch only [`crate::state::UiState`]
//! (plus read-only domain facts for clamping). The client owns these; the
//! daemon never sees them.

pub mod modals;
pub mod navigation;
pub mod presentation;
pub mod settings;

use crate::state::AppState;
use ratatube_domain::commands::UiMsg;
use ratatube_domain::effect::Effect;

/// Reduce one presentation message; every variant is client-local.
///
/// This is the single routing point for [`UiMsg`] and matches wildcard-free
/// on purpose: a new variant cannot compile until it is given an owner here.
/// Each transition below takes only the state and payload it needs, so a
/// sub-reducer can no longer be handed — and silently drop — a message that
/// belongs to another family.
pub fn reduce(state: &mut AppState, msg: UiMsg) -> Vec<Effect> {
    let ui = &mut state.ui;
    let domain = &state.domain;
    match msg {
        // Settings menu. `OpenSettings` is service-owned: it seeds the drafts
        // from configuration, so there is no UI-half transition for it.
        UiMsg::OpenSettings => {}
        UiMsg::CloseSettings => settings::close(ui),
        UiMsg::SettingsCycleTab => settings::cycle_tab(ui),
        UiMsg::SettingsMove(delta) => settings::move_selection(ui, delta),
        UiMsg::SettingsAdjust(delta) => settings::adjust(ui, delta),
        UiMsg::SettingsSubmit => return settings::submit(ui),

        // Presentation modes and notifications.
        UiMsg::ScrollNowPlaying(delta) => presentation::scroll_now_playing(ui, delta),
        UiMsg::ToggleNowPlayingPane => presentation::toggle_now_playing_pane(ui),
        UiMsg::CyclePlayingPane => presentation::cycle_playing_pane(ui, domain),
        UiMsg::ToggleHistoryViewMode => presentation::toggle_history_view_mode(ui, domain),
        UiMsg::DismissNotification => presentation::dismiss_notification(ui),
        UiMsg::ToggleNotificationLog => presentation::toggle_notification_log(ui),

        // View navigation, help, and selection.
        UiMsg::Navigate(view) => navigation::navigate(ui, domain, view),
        UiMsg::NextView => navigation::next_view(ui, domain),
        UiMsg::PreviousView => navigation::previous_view(ui, domain),
        UiMsg::OpenHelp => navigation::open_help(ui),
        UiMsg::CloseHelp => navigation::close_help(ui),
        UiMsg::ScrollHelp(delta) => navigation::scroll_help(ui, delta),
        UiMsg::CycleHomeSection(delta) => navigation::cycle_home_section(ui, domain, delta),
        UiMsg::ToggleSearchDetail => navigation::toggle_search_detail(ui),
        UiMsg::SelectNext => navigation::select_next(ui, domain),
        UiMsg::SelectPrevious => navigation::select_previous(ui),
    }
    Vec::new()
}
