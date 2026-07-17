//! Terminal UI: layout, theme, icons, widgets, and views.

pub mod icons;
pub mod layout;
pub mod theme;
pub mod views;
pub mod widgets;

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
    let icons = icons::icons_for(state_icon_mode(state));
    let theme = theme::Theme::default();
    terminal
        .draw(|frame| {
            let area = frame.area();
            state.screen_area = area;
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
            // The Playing view renders its own full playback panel, so the
            // compact bar would duplicate it; suppress it there.
            let has_bar = state.has_now_playing()
                && state.view != crate::app::state::View::NowPlaying;
            let regions = AppLayout::new(area, has_bar, true);
            widgets::render_header(frame, regions.header, state, &icons, &theme);
            views::render_main(frame, regions.main, state, history, &icons, &theme);
            widgets::render_now_playing(frame, regions.now_playing, state, &icons, &theme);
            widgets::render_footer(frame, regions.footer, state, &theme);

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

            if let Some(notification) = &state.notification {
                let style = if notification.is_error {
                    theme.error
                } else {
                    theme.accent
                };
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        icons::sanitize_terminal_text(&notification.message),
                        style,
                    )),
                    regions.header,
                );
            }

            render_overlays(frame, regions.main, state, &theme);
        })
        .map_err(|e| crate::error::AppError::Config(format!("render failed: {e}")))?;
    Ok(())
}

/// Render modal overlays on top of the main area: import progress/review,
/// text prompts, and confirmation dialogs.
fn render_overlays(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    theme: &theme::Theme,
) {
    use crate::app::state::ImportState;
    use ratatui::text::Line;
    use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

    let modal_area = centered_rect(area, 64, 14);
    let modal_block = |title: &str| {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border_active)
            .title(format!(" {title} "))
    };

    if let Some(import) = &state.import {
        let lines: Vec<Line> = match import {
            ImportState::Fetching { url } => vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("{} ", widgets::spinner(state.spinner_frame)),
                        theme.accent,
                    ),
                    Span::raw("Importing playlist..."),
                ]),
                Line::from(Span::styled(icons::sanitize_terminal_text(url), theme.dim)),
            ],
            ImportState::Review { summary, playlist } => vec![
                Line::from(Span::styled(
                    format!("\"{}\"", sanitize(&summary.remote_title)),
                    theme.header,
                )),
                Line::from(""),
                Line::from(format!("Total entries:     {}", summary.total_entries)),
                Line::from(vec![
                    Span::raw("Imported:          "),
                    Span::styled(format!("{}", summary.imported), theme.playing),
                ]),
                Line::from(vec![
                    Span::raw("Unavailable:       "),
                    Span::styled(format!("{}", summary.unavailable), theme.warning),
                ]),
                Line::from(format!("Duplicates:        {}", summary.duplicates)),
                Line::from(format!("Missing metadata:  {}", summary.missing_metadata)),
                Line::from(""),
                Line::from(format!("Save as: {}", sanitize(&playlist.name))),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" Enter ", theme.key_chip),
                    Span::styled("save    ", theme.dim),
                    Span::styled(" Esc ", theme.key_chip),
                    Span::styled("cancel", theme.dim),
                ]),
            ],
        };
        frame.render_widget(Clear, modal_area);
        frame.render_widget(
            Paragraph::new(lines).block(modal_block("Import playlist")),
            modal_area,
        );
        return;
    }

    if let Some(prompt) = &state.prompt {
        let title = match prompt.purpose {
            crate::app::state::PromptPurpose::SaveQueueAsPlaylist => "Save queue as playlist",
            crate::app::state::PromptPurpose::RenamePlaylist => "Rename playlist",
            crate::app::state::PromptPurpose::ImportPlaylistUrl => "Import playlist URL",
            crate::app::state::PromptPurpose::NewPlaylist => "New playlist name",
        };
        let prompt_area = centered_rect(area, 56, 5);
        let line = Line::from(vec![
            Span::raw(sanitize(&prompt.buffer)),
            Span::styled("▏", theme.accent),
        ]);
        frame.render_widget(Clear, prompt_area);
        frame.render_widget(Paragraph::new(line).block(modal_block(title)), prompt_area);
        return;
    }

    if let Some(confirm) = &state.confirm {
        let confirm_area = centered_rect(area, 50, 5);
        frame.render_widget(Clear, confirm_area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(sanitize(&confirm.message)),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" y ", theme.key_chip),
                    Span::styled("yes   ", theme.dim),
                    Span::styled(" n ", theme.key_chip),
                    Span::styled("no", theme.dim),
                ]),
            ])
            .block(modal_block("Confirm")),
            confirm_area,
        );
    }
}

fn sanitize(input: &str) -> String {
    icons::sanitize_terminal_text(input)
}

/// Center a modal of `width`/`height` within `area`.
fn centered_rect(area: ratatui::layout::Rect, width: u16, height: u16) -> ratatui::layout::Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    ratatui::layout::Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w,
        height: h,
    }
}

fn state_icon_mode(state: &AppState) -> crate::config::IconMode {
    state.icon_mode
}
