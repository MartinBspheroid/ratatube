//! Data-backed and placeholder panels for the Home dashboard.

use super::*;
use crate::app::state::HomeSection;
use crate::ui::components::{NumberedRow, header_link, numbered_row, section_panel};

pub(super) fn panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
    link: Option<(&str, char)>,
    icons: &Icons,
    theme: &Theme,
) -> Rect {
    let inner = section_panel(frame, area, title, focused, theme, icons);
    if let Some((label, key)) = link {
        frame.render_widget(
            Paragraph::new(Line::from(header_link(label, Some(key), theme))).right_aligned(),
            Rect { height: 1, ..area },
        );
    }
    inner
}

pub(super) fn render_recent(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    recent: &[crate::media::Track],
    total: usize,
    icons: &Icons,
    theme: &Theme,
) {
    let link = format!("View all ({total})");
    let inner = panel(
        frame,
        area,
        "Recent Tracks",
        state.ui.home_section == HomeSection::Recent,
        Some((&link, '5')),
        icons,
        theme,
    );
    let lines = recent
        .iter()
        .enumerate()
        .take(inner.height as usize)
        .map(|(index, track)| {
            let right = track
                .duration_seconds
                .map(|seconds| format_time(seconds as f64))
                .into_iter()
                .collect::<Vec<_>>();
            numbered_row(
                NumberedRow {
                    index,
                    title: &format!(
                        "{} — {}",
                        sanitize_terminal_text(&track.title),
                        sanitize_terminal_text(&track.artist)
                    ),
                    right_columns: &right,
                    playing: state
                        .domain
                        .current_track
                        .as_ref()
                        .is_some_and(|current| current.id == track.id),
                    selected: state.ui.home_section == HomeSection::Recent
                        && state.ui.selected_index == index,
                },
                inner.width as usize,
                theme,
                icons,
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(super) fn render_playlists(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = panel(
        frame,
        area,
        "Playlists",
        state.ui.home_section == HomeSection::Playlists,
        Some(("View all", '4')),
        icons,
        theme,
    );
    if inner.width >= crate::ui::layout::HOME_GRID_MIN_WIDTH {
        let page_count = state.domain.playlists.len().max(1).div_ceil(4);
        let page = (state.ui.selected_index / 4).min(page_count - 1);
        if page_count > 1 {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(
                        "{} {}/{} {}",
                        icons.chevron_l,
                        page + 1,
                        page_count,
                        icons.chevron_r
                    ),
                    theme.dim,
                )))
                .centered(),
                Rect { height: 1, ..area },
            );
        }
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25); 4])
            .split(inner);
        for ((index, playlist), column) in state
            .domain
            .playlists
            .iter()
            .enumerate()
            .skip(page * 4)
            .zip(columns.iter())
            .take(4)
        {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(icons.music, theme.accent)).centered(),
                    Line::from(Span::styled(
                        crate::ui::widgets::truncate_end(
                            &sanitize_terminal_text(&playlist.name),
                            column.width as usize,
                        ),
                        if state.ui.home_section == HomeSection::Playlists
                            && state.ui.selected_index == index
                        {
                            theme.selected
                        } else {
                            theme.base
                        },
                    ))
                    .centered(),
                    Line::from(Span::styled(
                        format!("{} tracks", playlist.tracks.len()),
                        theme.dim,
                    ))
                    .centered(),
                ]),
                *column,
            );
        }
    } else {
        let lines = state
            .domain
            .playlists
            .iter()
            .enumerate()
            .take(inner.height as usize)
            .map(|(index, playlist)| {
                numbered_row(
                    NumberedRow {
                        index,
                        title: &sanitize_terminal_text(&playlist.name),
                        right_columns: &[format!("{} tracks", playlist.tracks.len())],
                        playing: false,
                        selected: state.ui.home_section == HomeSection::Playlists
                            && state.ui.selected_index == index,
                    },
                    inner.width as usize,
                    theme,
                    icons,
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), inner);
    }
}
