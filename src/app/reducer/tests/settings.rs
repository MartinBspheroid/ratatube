use super::*;
use crate::app::state::{SETTINGS_GENERAL_ROWS, SettingsState, SettingsTab};
use crate::config::{IconMode, ResumeMode, ThemeName};

fn open_settings(state: &mut AppState) {
    state.ui.settings = Some(SettingsState {
        tab: SettingsTab::Appearance,
        selected: 0,
        original_theme: ThemeName::Neon,
        icons: IconMode::Auto,
        resume: ResumeMode::Paused,
    });
}

fn settings(state: &AppState) -> &SettingsState {
    state.ui.settings.as_ref().expect("settings open")
}

#[test]
fn moving_the_appearance_selection_previews_the_theme_live() {
    let mut state = AppState::new();
    open_settings(&mut state);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsMove(1)),
    );
    assert_eq!(settings(&state).selected, 1);
    assert_eq!(state.ui.theme, ThemeName::ALL[1]);
}

#[test]
fn appearance_selection_clamps_at_both_ends() {
    let mut state = AppState::new();
    open_settings(&mut state);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsMove(-1)),
    );
    assert_eq!(settings(&state).selected, 0);
    let last = ThemeName::ALL.len() - 1;
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsMove(last as i32 + 5)),
    );
    assert_eq!(settings(&state).selected, last);
}

#[test]
fn closing_the_menu_restores_the_opening_theme() {
    let mut state = AppState::new();
    open_settings(&mut state);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsMove(2)),
    );
    assert_ne!(state.ui.theme, ThemeName::Neon);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::CloseSettings),
    );
    assert!(state.ui.settings.is_none());
    assert_eq!(state.ui.theme, ThemeName::Neon);
}

#[test]
fn submit_persists_the_previewed_theme_and_closes() {
    let mut state = AppState::new();
    open_settings(&mut state);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsMove(1)),
    );
    let effects = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsSubmit),
    );
    assert!(state.ui.settings.is_none());
    assert_eq!(state.ui.theme, ThemeName::ALL[1]);
    assert_eq!(
        effects,
        vec![Effect::PersistUiSettings {
            theme: ThemeName::ALL[1],
            icons: IconMode::Auto,
            resume: ResumeMode::Paused,
        }]
    );
}

#[test]
fn tab_cycling_lands_general_on_the_first_row_and_back_on_the_theme() {
    let mut state = AppState::new();
    state.ui.theme = ThemeName::Nord;
    open_settings(&mut state);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsCycleTab),
    );
    assert_eq!(settings(&state).tab, SettingsTab::General);
    assert_eq!(settings(&state).selected, 0);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsCycleTab),
    );
    assert_eq!(settings(&state).tab, SettingsTab::Appearance);
    assert_eq!(
        settings(&state).selected,
        crate::app::reducer::theme_index(ThemeName::Nord)
    );
}

#[test]
fn general_rows_cycle_their_values_and_wrap() {
    let mut state = AppState::new();
    open_settings(&mut state);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsCycleTab),
    );
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsAdjust(-1)),
    );
    assert_eq!(settings(&state).icons, IconMode::Ascii);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsMove(1)),
    );
    assert_eq!(settings(&state).selected, SETTINGS_GENERAL_ROWS - 1);
    let _ = reduce(
        &mut state,
        Action::Navigation(NavigationAction::SettingsAdjust(1)),
    );
    assert_eq!(settings(&state).resume, ResumeMode::Playing);
    // Value changes on General never touch the live theme.
    assert_eq!(state.ui.theme, ThemeName::Neon);
}

#[test]
fn settings_actions_without_an_open_menu_are_no_ops() {
    let mut state = AppState::new();
    for action in [
        NavigationAction::CloseSettings,
        NavigationAction::SettingsCycleTab,
        NavigationAction::SettingsMove(1),
        NavigationAction::SettingsAdjust(1),
        NavigationAction::SettingsSubmit,
    ] {
        let effects = reduce(&mut state, Action::Navigation(action));
        assert!(effects.is_empty());
        assert!(state.ui.settings.is_none());
    }
}
