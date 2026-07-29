//! Playlists master-detail renderer.

use super::*;

use crate::render::components::{
    EmptyState, TrackRow, TrackTableLayout, empty_state, header_row, track_flags, track_row,
};
use crate::render::layout::Breakpoint;

pub(super) fn render_playlists(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    if state.domain.playlists.is_empty() {
        render_empty(frame, area, icons, theme);
        return;
    }
    let breakpoint = Breakpoint::from_width(area.width);
    let panes = if breakpoint == Breakpoint::Narrow {
        vec![area]
    } else {
        Layout::horizontal(crate::render::layout::PLAYLIST_MASTER.map(Constraint::Percentage))
            .spacing(crate::render::layout::PANE_GUTTER)
            .split(area)
            .to_vec()
    };
    render_master(frame, panes[0], state, icons, theme);
    if let Some(detail_area) = panes.get(1).copied() {
        render_selected(frame, detail_area, state, icons, theme);
    }
}

fn render_empty(frame: &mut Frame, area: Rect, icons: &Icons, theme: &Theme) {
    let inner = section_panel(frame, area, "Playlists (0)", true, theme, icons);
    empty_state(
        frame,
        inner,
        EmptyState {
            icon: icons.playlist,
            headline: "No playlists yet",
            hints: &[
                ("i", "import URL"),
                ("I", "paste JSON"),
                ("N", "new"),
                ("w", "save queue (in Queue)"),
            ],
        },
        theme,
    );
}

fn render_master(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let total = state.domain.playlists.len();
    let inner = section_panel(
        frame,
        area,
        &format!("Playlists ({total})"),
        true,
        theme,
        icons,
    );
    let inner = render_filter_bar(frame, inner, state, total, icons, theme);
    let selected = state.resolve_index(state.ui.selected_index);
    let items = visible_positions(state, total)
        .into_iter()
        .filter_map(|index| {
            let playlist = state.domain.playlists.get(index)?;
            let count = format!("{} tracks", playlist.tracks.len());
            Some(ListItem::new(numbered_row(
                NumberedRow {
                    index,
                    title: &sanitize_terminal_text(&playlist.name),
                    right_columns: &[count],
                    playing: false,
                    selected: selected == index,
                },
                inner.width.saturating_sub(1) as usize,
                theme,
                icons,
            )))
        })
        .collect::<Vec<_>>();
    state.ui.list_state.select(Some(state.ui.selected_index));
    state.ui.list_hit_area = inner;
    frame.render_stateful_widget(List::new(items), inner, &mut state.ui.list_state);
    scrollbar(frame, inner, total, state.ui.list_state.offset());
}

fn render_selected(frame: &mut Frame, area: Rect, state: &AppState, icons: &Icons, theme: &Theme) {
    let Some(playlist) = state
        .domain
        .playlists
        .get(state.resolve_index(state.ui.selected_index))
    else {
        return;
    };
    let rows = Layout::vertical([Constraint::Length(9), Constraint::Min(5)])
        .spacing(crate::render::layout::PANE_GUTTER)
        .split(area);
    render_info(frame, rows[0], playlist, icons, theme);
    render_preview(frame, rows[1], state, playlist, icons, theme);
}

fn render_info(
    frame: &mut Frame,
    area: Rect,
    playlist: &ratatube_domain::playlists::Playlist,
    icons: &Icons,
    theme: &Theme,
) {
    let inner = section_panel(frame, area, "Playlist info", false, theme, icons);
    let source = playlist.source.as_ref();
    let values = vec![
        ("Name".to_string(), sanitize_terminal_text(&playlist.name)),
        ("Tracks".to_string(), playlist.tracks.len().to_string()),
        (
            "Source URL".to_string(),
            source.map_or_else(
                || "Local".to_string(),
                |item| sanitize_terminal_text(&item.url),
            ),
        ),
        (
            "Last synced".to_string(),
            source.and_then(|item| item.last_synced_at).map_or_else(
                || "Never".to_string(),
                |at| at.format("%Y-%m-%d %H:%M UTC").to_string(),
            ),
        ),
        (
            "Updated".to_string(),
            playlist.updated_at.format("%Y-%m-%d %H:%M UTC").to_string(),
        ),
        (
            "Description".to_string(),
            if playlist.description.is_empty() {
                "None".to_string()
            } else {
                sanitize_terminal_text(&playlist.description)
            },
        ),
    ];
    let columns = Layout::horizontal([Constraint::Length(8), Constraint::Min(1)]).split(inner);
    let art = vec![
        Line::from(Span::styled(icons.playlist, theme.accent_alt)),
        Line::from(Span::styled(icons.panel_rule.repeat(5), theme.panel_rule)),
        Line::from(Span::styled(icons.music, theme.accent)),
    ];
    frame.render_widget(Paragraph::new(art).centered(), columns[0]);
    frame.render_widget(
        Paragraph::new(key_value_rows(&values, false, theme)),
        columns[1],
    );
}

fn render_preview(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    playlist: &ratatube_domain::playlists::Playlist,
    icons: &Icons,
    theme: &Theme,
) {
    let title = format!("Track preview ({})", playlist.tracks.len());
    let inner = section_panel(frame, area, &title, false, theme, icons);
    let layout = TrackTableLayout::new(inner.width, 8);
    let rows =
        playlist.tracks.iter().enumerate().map(|(index, track)| {
            track_row(&layout, preview_row(state, index, track), theme, icons)
        });
    frame.render_widget(
        Table::new(rows, layout.constraints()).header(header_row("LENGTH", theme)),
        inner,
    );
}

fn preview_row(
    state: &AppState,
    index: usize,
    track: &ratatube_domain::playlists::model::PlaylistTrack,
) -> TrackRow {
    TrackRow {
        index,
        title: sanitize_terminal_text(&track.title),
        channel: sanitize_terminal_text(&track.artist),
        right: track
            .duration_seconds
            .map(|seconds| format_time(seconds as f64))
            .unwrap_or_else(|| "--:--".to_string()),
        flags: track_flags(state, &track.id),
    }
}

pub(super) fn render_playlist_detail(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let Some(playlist) = state
        .ui
        .selected_playlist
        .and_then(|index| state.domain.playlists.get(index))
        .cloned()
    else {
        section_panel(frame, area, "Playlist", true, theme, icons);
        return;
    };
    let panel = section_panel(
        frame,
        area,
        &format!("Playlist editor ({})", playlist.tracks.len()),
        true,
        theme,
        icons,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(sanitize_terminal_text(&playlist.name), theme.header),
            Span::styled("  ·  ", theme.dim),
            Span::styled(
                if playlist.description.is_empty() {
                    "No description".to_string()
                } else {
                    sanitize_terminal_text(&playlist.description)
                },
                theme.dim,
            ),
        ])),
        Rect { height: 1, ..panel },
    );
    let inner = Rect {
        y: panel.y + 2,
        height: panel.height.saturating_sub(2),
        ..panel
    };
    let panes = if inner.width >= crate::render::layout::INSPECTOR_MIN_WIDTH {
        Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)])
            .spacing(crate::render::layout::PANE_GUTTER)
            .split(inner)
            .to_vec()
    } else {
        vec![inner]
    };
    let layout = TrackTableLayout::new(panes[0].width, 8);
    let table_rows =
        playlist.tracks.iter().enumerate().map(|(index, track)| {
            track_row(&layout, preview_row(state, index, track), theme, icons)
        });
    let table = Table::new(table_rows, layout.constraints())
        .header(header_row("LENGTH", theme))
        .row_highlight_style(theme.selected)
        .highlight_symbol(icons.chevron_r);
    state.ui.table_state.select(Some(state.ui.selected_index));
    state.ui.list_hit_area = Rect {
        y: panes[0].y + 2,
        height: panes[0].height.saturating_sub(2),
        ..panes[0]
    };
    frame.render_stateful_widget(table, panes[0], &mut state.ui.table_state);
    scrollbar(
        frame,
        panes[0],
        playlist.tracks.len(),
        state.ui.table_state.offset(),
    );
    render_inspector(
        frame,
        &panes,
        &playlist,
        state.ui.selected_index,
        theme,
        icons,
    );
}

fn render_inspector(
    frame: &mut Frame,
    panes: &[Rect],
    playlist: &ratatube_domain::playlists::Playlist,
    selected_index: usize,
    theme: &Theme,
    icons: &Icons,
) {
    if let Some(inspector) = panes.get(1).copied() {
        let selected = playlist.tracks.get(selected_index);
        let inspector_inner =
            section_panel(frame, inspector, "Selected track", false, theme, icons);
        let lines = selected.map_or_else(
            || vec![Line::from(Span::styled("No track selected", theme.dim))],
            |track| {
                vec![
                    Line::from(Span::styled(
                        sanitize_terminal_text(&track.title),
                        theme.header,
                    )),
                    Line::from(""),
                    Line::from(Span::styled("CHANNEL", theme.label)),
                    Line::from(sanitize_terminal_text(&track.artist)),
                    Line::from(""),
                    Line::from(Span::styled("VIDEO URL", theme.label)),
                    Line::from(Span::styled(
                        crate::render::widgets::truncate_middle(
                            &sanitize_terminal_text(&track.webpage_url),
                            inspector_inner.width as usize,
                        ),
                        theme.dim,
                    )),
                ]
            },
        );
        frame.render_widget(
            Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false }),
            inspector_inner,
        );
    }
}
