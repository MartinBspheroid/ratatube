//! The ctrl+p settings menu: tab movement, live theme preview, and the
//! save/cancel transitions.

use crate::app::action::NavigationAction;
use crate::app::reducer::Effect;
use crate::app::state::{SETTINGS_GENERAL_ROWS, SettingsTab, UiState};
use crate::config::{IconMode, ResumeMode, ThemeName};

/// Reduce settings-menu transitions. `OpenSettings` is service-owned (it
/// seeds drafts from configuration) and is a no-op here.
pub(in crate::app::reducer) fn reduce(ui: &mut UiState, action: NavigationAction) -> Vec<Effect> {
    match action {
        NavigationAction::CloseSettings => {
            if let Some(settings) = ui.settings.take() {
                ui.theme = settings.original_theme;
            }
        }
        NavigationAction::SettingsCycleTab => {
            if let Some(settings) = &mut ui.settings {
                (settings.tab, settings.selected) = match settings.tab {
                    SettingsTab::Appearance => (SettingsTab::General, 0),
                    SettingsTab::General => (SettingsTab::Appearance, theme_index(ui.theme)),
                };
            }
        }
        NavigationAction::SettingsMove(delta) => {
            if let Some(settings) = &mut ui.settings {
                let rows = match settings.tab {
                    SettingsTab::Appearance => ThemeName::ALL.len(),
                    SettingsTab::General => SETTINGS_GENERAL_ROWS,
                };
                settings.selected = settings
                    .selected
                    .saturating_add_signed(delta as isize)
                    .min(rows - 1);
                if settings.tab == SettingsTab::Appearance {
                    ui.theme = ThemeName::ALL[settings.selected];
                }
            }
        }
        NavigationAction::SettingsAdjust(delta) => {
            if let Some(settings) = &mut ui.settings
                && settings.tab == SettingsTab::General
            {
                match settings.selected {
                    0 => settings.icons = cycled(&IconMode::ALL, settings.icons, delta),
                    _ => settings.resume = cycled(&ResumeMode::ALL, settings.resume, delta),
                }
            }
        }
        NavigationAction::SettingsSubmit => {
            if let Some(settings) = ui.settings.take() {
                return vec![Effect::PersistUiSettings {
                    theme: ui.theme,
                    icons: settings.icons,
                    resume: settings.resume,
                }];
            }
        }
        // `OpenSettings` and every non-settings variant.
        _ => {}
    }
    Vec::new()
}

/// Position of `theme` in the settings-menu order.
pub(crate) fn theme_index(theme: ThemeName) -> usize {
    ThemeName::ALL
        .iter()
        .position(|&candidate| candidate == theme)
        .unwrap_or(0)
}

/// Step `current` through `all` by `delta`, wrapping at both ends.
fn cycled<T: Copy + PartialEq>(all: &[T], current: T, delta: i32) -> T {
    let len = all.len() as i32;
    let index = all
        .iter()
        .position(|&candidate| candidate == current)
        .unwrap_or(0) as i32;
    all[(index + delta).rem_euclid(len) as usize]
}
