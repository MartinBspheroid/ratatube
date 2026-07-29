//! UI-half reducers: transitions that touch only [`UiState`] (plus read-only
//! domain facts for clamping). The client owns these; the daemon never sees
//! them.

pub mod modals;
pub mod navigation;
pub mod presentation;
pub mod settings;

use crate::state::AppState;
use ratatube_domain::commands::UiMsg;
use ratatube_domain::effect::Effect;

/// Reduce one presentation message; every variant is client-local.
pub fn reduce(state: &mut AppState, msg: UiMsg) -> Vec<Effect> {
    match msg {
        UiMsg::OpenSettings
        | UiMsg::CloseSettings
        | UiMsg::SettingsCycleTab
        | UiMsg::SettingsMove(_)
        | UiMsg::SettingsAdjust(_)
        | UiMsg::SettingsSubmit => settings::reduce(&mut state.ui, msg),
        UiMsg::ScrollNowPlaying(_)
        | UiMsg::ToggleNowPlayingPane
        | UiMsg::CyclePlayingPane
        | UiMsg::ToggleHistoryViewMode
        | UiMsg::DismissNotification
        | UiMsg::ToggleNotificationLog => presentation::reduce(&mut state.ui, &state.domain, msg),
        UiMsg::Navigate(_)
        | UiMsg::OpenHelp
        | UiMsg::CloseHelp
        | UiMsg::ScrollHelp(_)
        | UiMsg::NextView
        | UiMsg::PreviousView
        | UiMsg::CycleHomeSection(_)
        | UiMsg::ToggleSearchDetail
        | UiMsg::SelectNext
        | UiMsg::SelectPrevious => navigation::reduce(&mut state.ui, &state.domain, msg),
    }
}
