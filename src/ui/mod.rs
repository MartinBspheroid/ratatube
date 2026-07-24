//! Terminal UI: layout, theme, icons, widgets, and views.

pub mod activity;
pub mod components;
pub mod context_menu;
pub mod footer;
pub mod header;
pub mod icons;
pub mod layout;
pub mod theme;
pub mod views;
pub mod widgets;

mod overlay_playlists;
mod overlay_settings;
mod overlay_status;
mod overlays;

use ratatui::DefaultTerminal;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::state::AppState;
use crate::error::Result;
use crate::history::HistoryService;
use crate::ui::layout::{AppLayout, is_compact};

/// Alias for the concrete terminal type used by the app.
pub type Terminal = DefaultTerminal;

/// Render one frame from application state.
pub fn render<B>(terminal: &mut ratatui::Terminal<B>, state: &mut AppState) -> Result<()>
where
    B: ratatui::backend::Backend,
    B::Error: std::fmt::Display,
{
    render_with(terminal, state, None)
}

/// Render one frame, optionally with access to history for the history view.
pub fn render_with<B>(
    terminal: &mut ratatui::Terminal<B>,
    state: &mut AppState,
    history: Option<&HistoryService>,
) -> Result<()>
where
    B: ratatui::backend::Backend,
    B::Error: std::fmt::Display,
{
    let icons = icons::icons_for(state.ui.icon_mode);
    let theme = theme::Theme::resolve(state.ui.theme);
    terminal
        .draw(|frame| {
            let area = frame.area();
            state.ui.screen_area = area;
            frame.buffer_mut().set_style(area, theme.bg);
            if is_compact(area) {
                // Below the hard floor nothing useful fits; warn instead of
                // panicking (PRD 20).
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!(
                            "Terminal too small ({}x{}; need at least {}x{})",
                            area.width,
                            area.height,
                            layout::FLOOR_COLS,
                            layout::FLOOR_ROWS
                        ),
                        theme.error,
                    )),
                    area,
                );
                return;
            }
            let regions = AppLayout::new(area, state.has_now_playing(), true);
            header::render_header(frame, regions.header, state, &icons, &theme);
            views::render_main(frame, regions.main, state, history, &icons, &theme);
            widgets::render_now_playing(frame, regions.now_playing, state, &icons, &theme);
            footer::render_footer(frame, regions.footer, state, &theme);

            // Below the documented minimum, note it at the bottom-right; the UI
            // stays fully usable (PRD 20).
            if layout::is_small(area) {
                let note = Line::from(Span::styled(
                    format!(
                        "small terminal — best at {}x{}",
                        layout::MIN_COLS,
                        layout::MIN_ROWS
                    ),
                    theme.warning,
                ))
                .right_aligned();
                let note_area = ratatui::layout::Rect {
                    x: area.x,
                    y: area.y + area.height.saturating_sub(1),
                    width: area.width,
                    height: 1,
                };
                frame.render_widget(Paragraph::new(note), note_area);
            }

            // Modal content stays below notifications so operation failures
            // remain visible while the context menu is open.
            overlays::render(frame, regions.main, state, &icons, &theme);
            if let Some(notification) = &state.ui.notification {
                let style = if notification.is_error {
                    theme.error
                } else {
                    theme.accent
                };
                // Fixed chrome slot: right-aligned and bounded to half the
                // header so the logo and tabs stay visible underneath.
                let budget = usize::from((regions.header.width / 2).max(24));
                let message = widgets::truncate_end(
                    &icons::sanitize_terminal_text(&notification.message),
                    budget,
                );
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(message, style)).right_aligned()),
                    regions.header,
                );
            }
        })
        .map_err(|error| crate::error::AppError::Config(format!("render failed: {error}")))?;
    Ok(())
}

/// Center a modal of `width`/`height` within `area`.
pub(crate) fn centered_rect(
    area: ratatui::layout::Rect,
    width: u16,
    height: u16,
) -> ratatui::layout::Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    ratatui::layout::Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}
