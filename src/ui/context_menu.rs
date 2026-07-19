//! Universal selected-track context menu renderer.

use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::state::{TrackContextMenuState, TrackDetailsModalState};
use crate::app::track_context::TrackContextAction;
use crate::ui::{centered_rect, icons, theme};

/// Render the non-blocking track context menu above the active view.
pub fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    menu: &TrackContextMenuState,
    theme: &theme::Theme,
) {
    let height = (menu.context.actions.len() as u16 + 2)
        .max(4)
        .min(area.height);
    let menu_area = centered_rect(area, 56, height);
    let title = icons::sanitize_terminal_text(&menu.context.track.title);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border_active)
        .title(format!(" Track — {title} "))
        .title_bottom(Line::from(vec![
            Span::styled(" Enter ", theme.key_chip),
            Span::styled("select   ", theme.dim),
            Span::styled(" j/k ", theme.key_chip),
            Span::styled("move   ", theme.dim),
            Span::styled(" Esc ", theme.key_chip),
            Span::styled("close", theme.dim),
        ]));
    let items: Vec<ListItem> = menu
        .context
        .actions
        .iter()
        .map(|action| {
            let (label, warning) = action_label(action);
            let prefix = if warning { "! " } else { "  " };
            let style = if warning { theme.warning } else { theme.base };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{label}"), style)))
        })
        .collect();
    let selected = if menu.context.actions.is_empty() {
        None
    } else {
        Some(menu.selected % menu.context.actions.len())
    };
    let mut list_state = ListState::default().with_selected(selected);

    frame.render_widget(Clear, menu_area);
    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(theme.selected),
        menu_area,
        &mut list_state,
    );
}

/// Render details for the context-selected track without replacing playback data.
pub fn render_details(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    modal: &TrackDetailsModalState,
    theme: &theme::Theme,
) {
    let detail_area = centered_rect(area, 68, 14);
    let track = &modal.track;
    let duration = track.duration_seconds.map_or_else(
        || "--:--".to_string(),
        |value| crate::ui::widgets::format_time(value as f64),
    );
    let mut lines = vec![
        Line::from(Span::styled(
            icons::sanitize_terminal_text(&track.title),
            theme.header,
        )),
        Line::from(""),
        detail_line("Artist", &track.artist, theme),
        detail_line("Duration", &duration, theme),
        detail_line("Video URL", &track.webpage_url, theme),
    ];
    if let Some(details) = &modal.details {
        if let Some(views) = details.view_count {
            lines.push(detail_line(
                "Views",
                &crate::media::format_count(views),
                theme,
            ));
        }
        if let Some(uploaded) = details.formatted_upload_date() {
            lines.push(detail_line("Uploaded", &uploaded, theme));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Esc ", theme.key_chip),
        Span::styled("close", theme.dim),
    ]));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border_active)
        .title(" Track details ");
    frame.render_widget(Clear, detail_area);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(block),
        detail_area,
    );
}

fn detail_line(label: &'static str, value: &str, theme: &theme::Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), theme.dim),
        Span::styled(icons::sanitize_terminal_text(value), theme.base),
    ])
}

fn action_label(action: &TrackContextAction) -> (&'static str, bool) {
    match action {
        TrackContextAction::PlayNow => ("Play now", false),
        TrackContextAction::PlayNext => ("Play next", false),
        TrackContextAction::AddToQueue => ("Add to queue", false),
        TrackContextAction::AddToPlaylist => ("Add to playlist", false),
        TrackContextAction::VisitChannel => ("Visit channel", false),
        TrackContextAction::ShowDetails => ("Show details", false),
        TrackContextAction::OpenInBrowser => ("Open in browser", false),
        TrackContextAction::CopyUrl => ("Copy URL", false),
        TrackContextAction::RemoveFromQueue { .. } => ("Remove from queue", true),
        TrackContextAction::RemoveFromPlaylist { .. } => ("Remove from playlist", true),
    }
}
