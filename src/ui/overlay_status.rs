//! Notification-log and import overlays.

use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::app::state::{AppState, ImportState};
use crate::ui::{centered_rect, icons, theme, widgets};

/// Render status-owned overlays, returning whether one was active.
pub(super) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    theme: &theme::Theme,
) -> bool {
    if state.show_notification_log {
        let height = (state.notification_log.len() as u16 + 3).clamp(5, area.height);
        let log_area = centered_rect(area, 76, height);
        let mut lines: Vec<Line> = state
            .notification_log
            .iter()
            .take(height.saturating_sub(3) as usize)
            .map(|notification| {
                Line::from(vec![
                    Span::styled(
                        notification.created_at.format("%H:%M:%S ").to_string(),
                        theme.dim,
                    ),
                    Span::styled(
                        if notification.is_error {
                            "ERROR "
                        } else {
                            "INFO  "
                        },
                        if notification.is_error {
                            theme.error
                        } else {
                            theme.dim
                        },
                    ),
                    Span::styled(
                        icons::sanitize_terminal_text(&notification.message),
                        if notification.is_error {
                            theme.error
                        } else {
                            theme.base
                        },
                    ),
                ])
            })
            .collect();
        if lines.is_empty() {
            lines.push(Line::from(Span::styled("No messages yet", theme.dim)));
        }
        lines.push(Line::from(Span::styled("Esc close", theme.dim)));
        frame.render_widget(Clear, log_area);
        frame.render_widget(
            Paragraph::new(lines).block(modal_block("Messages (newest first)", theme)),
            log_area,
        );
        return true;
    }

    let Some(import) = &state.import else {
        return false;
    };
    let lines: Vec<Line> = match import {
        ImportState::Fetching { url, .. } => vec![
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
                format!(
                    "\"{}\"",
                    icons::sanitize_terminal_text(&summary.remote_title)
                ),
                theme.header,
            )),
            Line::from(""),
            Line::from(format!("Total entries:     {}", summary.total_entries)),
            Line::from(vec![
                Span::raw("Imported:          "),
                Span::styled(summary.imported.to_string(), theme.playing),
            ]),
            warning_count("Deleted:           ", summary.deleted, theme),
            warning_count("Private:           ", summary.private, theme),
            warning_count("Unavailable:       ", summary.unavailable, theme),
            Line::from(format!("Duplicates:        {}", summary.duplicates)),
            Line::from(format!("Missing ID:        {}", summary.missing_id)),
            Line::from(format!("Missing title:     {}", summary.missing_title)),
            Line::from(""),
            Line::from(format!(
                "Save as: {}",
                icons::sanitize_terminal_text(&playlist.name)
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Enter ", theme.key_chip),
                Span::styled("save    ", theme.dim),
                Span::styled(" Esc ", theme.key_chip),
                Span::styled("cancel", theme.dim),
            ]),
        ],
        ImportState::Failed { url, message } => vec![
            Line::from("Import failed"),
            Line::from(url.clone()),
            Line::from(message.clone()),
            Line::from("Press Esc to close, then retry from Playlists."),
        ],
    };
    let modal_area = centered_rect(area, 64, 18);
    frame.render_widget(Clear, modal_area);
    frame.render_widget(
        Paragraph::new(lines).block(modal_block("Import playlist", theme)),
        modal_area,
    );
    true
}

fn warning_count(label: &'static str, count: usize, theme: &theme::Theme) -> Line<'static> {
    Line::from(vec![
        Span::raw(label),
        Span::styled(count.to_string(), theme.warning),
    ])
}

fn modal_block<'a>(title: &'a str, theme: &theme::Theme) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border_active)
        .title(format!(" {title} "))
}
