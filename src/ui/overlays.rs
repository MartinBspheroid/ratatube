//! Modal overlay ordering.

use crate::app::state::AppState;
use crate::ui::{context_menu, icons, overlay_playlists, overlay_settings, overlay_status, theme};

/// Render the highest-priority active overlay.
pub(super) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    icon_set: &icons::Icons,
    theme: &theme::Theme,
) {
    if let Some(menu) = &state.ui.track_context_menu {
        context_menu::render(frame, area, menu, theme);
        return;
    }
    if let Some(details) = &state.ui.track_details_modal {
        context_menu::render_details(frame, area, details, theme);
        return;
    }
    if let Some(settings) = &state.ui.settings {
        overlay_settings::render(frame, area, settings, theme);
        return;
    }
    if overlay_status::render(frame, area, state, theme) {
        return;
    }
    overlay_playlists::render(frame, area, state, icon_set, theme);
}
