//! Hero and metadata panels for the responsive Playing view.

use super::*;
use crate::ui::components::{chips, key_value_rows, section_panel};
use crate::ui::layout::Breakpoint;

pub(super) fn render_hero(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    track: &crate::media::Track,
    breakpoint: Breakpoint,
    icons: &Icons,
    theme: &Theme,
) {
    let show_art = breakpoint != Breakpoint::Narrow;
    let show_metadata = breakpoint >= Breakpoint::Wide;
    let constraints = match (show_art, show_metadata) {
        (false, _) => vec![Constraint::Percentage(100)],
        (true, false) => vec![Constraint::Length(20), Constraint::Min(20)],
        (true, true) => vec![
            Constraint::Length(20),
            Constraint::Min(32),
            Constraint::Length(28),
        ],
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .spacing(crate::ui::layout::PANE_GUTTER)
        .split(area);

    let mut info_index = 0;
    if show_art {
        render_art(frame, columns[0], state, icons, theme);
        info_index = 1;
    }
    let info = columns[info_index].inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    render_track_info(frame, info, state, track, theme);

    if show_metadata {
        let metadata = columns[columns.len() - 1];
        render_metadata(frame, metadata, state, track, icons, theme);
    }
}

fn render_art(frame: &mut Frame, area: Rect, state: &mut AppState, icons: &Icons, theme: &Theme) {
    if let Some(protocol) = state.ui.thumbnail.as_mut() {
        let image =
            ratatui_image::StatefulImage::<ratatui_image::protocol::StatefulProtocol>::new();
        frame.render_stateful_widget(image, area, protocol);
    } else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(icons.music, theme.accent)).centered(),
            ])
            .style(theme.panel_bg),
            area,
        );
    }
}

fn render_track_info(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    track: &crate::media::Track,
    theme: &Theme,
) {
    let details = state.domain.current_details.as_ref();
    let width = area.width as usize;
    let mut lines = vec![
        Line::from(Span::styled(
            crate::ui::widgets::truncate_middle(&sanitize_terminal_text(&track.title), width),
            theme.header,
        )),
        Line::from(Span::styled(
            crate::ui::widgets::truncate_end(&sanitize_terminal_text(&track.artist), width),
            theme.accent,
        )),
        Line::from(""),
        stats_line(details, theme),
    ];
    let chip_values = detail_chips(details);
    if !chip_values.is_empty() {
        lines.push(chips(&chip_values, theme));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn stats_line(details: Option<&crate::media::TrackDetails>, theme: &Theme) -> Line<'static> {
    let Some(details) = details else {
        return Line::from(Span::styled("Metadata loading or unavailable", theme.dim));
    };
    let mut values = Vec::new();
    if let Some(views) = details.view_count {
        values.push(format!("{} views", crate::media::format_count(views)));
    }
    if let Some(likes) = details.like_count {
        values.push(format!("{} likes", crate::media::format_count(likes)));
    }
    if let Some(date) = details.formatted_upload_date() {
        values.push(date);
    }
    Line::from(Span::styled(values.join("  "), theme.dim))
}

fn detail_chips(details: Option<&crate::media::TrackDetails>) -> Vec<String> {
    let Some(details) = details else {
        return Vec::new();
    };
    let mut values = Vec::new();
    if let Some(codec) = &details.acodec {
        values.push(codec.to_uppercase());
    }
    if let Some(bitrate) = details.abr {
        values.push(format!("{bitrate:.0}kbps"));
    }
    if let Some(rate) = details.asr {
        values.push(format!("{rate}Hz"));
    }
    if let Some(channels) = details.audio_channels {
        values.push(match channels {
            1 => "Mono".to_string(),
            2 => "Stereo".to_string(),
            count => format!("{count}ch"),
        });
    }
    values
}

fn render_metadata(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    track: &crate::media::Track,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, "Metadata", false, theme, icons);
    let details = state.domain.current_details.as_ref();
    let mut pairs = vec![
        ("SOURCE".to_string(), "YouTube".to_string()),
        (
            "CHANNEL".to_string(),
            details
                .and_then(|value| value.uploader.clone())
                .unwrap_or_else(|| sanitize_terminal_text(&track.artist)),
        ),
    ];
    if let Some(published) = details.and_then(crate::media::TrackDetails::formatted_upload_date) {
        pairs.push(("PUBLISHED".to_string(), published));
    }
    if let Some(bitrate) = details.and_then(|value| value.abr) {
        pairs.push(("BITRATE".to_string(), format!("{bitrate:.0} kbps")));
    }
    frame.render_widget(Paragraph::new(key_value_rows(&pairs, false, theme)), inner);
}
