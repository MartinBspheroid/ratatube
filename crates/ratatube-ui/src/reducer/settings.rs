//! The ctrl+p settings menu: tab movement, live theme preview, and the
//! save/cancel transitions. On the Appearance tab the selection walks theme
//! families while h/l flips the dark/light mode of the whole preview.
//!
//! Each transition takes only the state and payload it needs; the parent
//! [`crate::reducer::reduce`] owns the wildcard-free routing, so no settings
//! transition can be reached by a message it does not belong to.

use crate::state::{SETTINGS_GENERAL_ROWS, SettingsTab, UiState};
use ratatube_domain::config::{IconMode, ResumeMode, ThemeFamily};
use ratatube_domain::effect::Effect;

/// Close the settings menu, restoring the theme it opened with.
pub(super) fn close(ui: &mut UiState) {
    if let Some(settings) = ui.settings.take() {
        ui.theme = settings.original_theme;
    }
}

/// Move to the next settings tab, seeding its selection.
pub(super) fn cycle_tab(ui: &mut UiState) {
    if let Some(settings) = &mut ui.settings {
        (settings.tab, settings.selected) = match settings.tab {
            SettingsTab::Appearance => (SettingsTab::General, 0),
            SettingsTab::General => (SettingsTab::Appearance, family_index(ui.theme.family())),
        };
    }
}

/// Move the settings selection by signed rows, previewing themes live.
pub(super) fn move_selection(ui: &mut UiState, delta: i32) {
    if let Some(settings) = &mut ui.settings {
        let rows = match settings.tab {
            SettingsTab::Appearance => ThemeFamily::ALL.len(),
            SettingsTab::General => SETTINGS_GENERAL_ROWS,
        };
        settings.selected = settings
            .selected
            .saturating_add_signed(delta as isize)
            .min(rows - 1);
        if settings.tab == SettingsTab::Appearance {
            ui.theme = ThemeFamily::ALL[settings.selected].variant(ui.theme.mode());
        }
    }
}

/// Cycle the selected settings value by a signed step.
pub(super) fn adjust(ui: &mut UiState, delta: i32) {
    if let Some(settings) = &mut ui.settings {
        match settings.tab {
            // Both directions flip between the two modes.
            SettingsTab::Appearance => {
                ui.theme = ui.theme.family().variant(ui.theme.mode().toggled());
            }
            // The General tab has `SETTINGS_GENERAL_ROWS` rows: row 0 is the
            // icon mode, row 1 the resume mode. `move_selection` clamps the
            // selection into that range; a third row means extending this.
            SettingsTab::General => {
                if settings.selected == 0 {
                    settings.icons = cycled(&IconMode::ALL, settings.icons, delta);
                } else {
                    settings.resume = cycled(&ResumeMode::ALL, settings.resume, delta);
                }
            }
        }
    }
}

/// Persist every settings draft and close the menu.
pub(super) fn submit(ui: &mut UiState) -> Vec<Effect> {
    match ui.settings.take() {
        Some(settings) => vec![Effect::PersistUiSettings {
            theme: ui.theme,
            icons: settings.icons,
            resume: settings.resume,
        }],
        None => Vec::new(),
    }
}

/// Position of `family` in the settings-menu order.
pub fn family_index(family: ThemeFamily) -> usize {
    ThemeFamily::ALL
        .iter()
        .position(|&candidate| candidate == family)
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
