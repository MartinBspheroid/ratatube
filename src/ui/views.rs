//! View renderers for each primary screen (PRD section 9).
//!
//! Views render from [`AppState`] and mutate only `list_state` so ratatui
//! can scroll lists naturally; all data changes flow through actions.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table,
};

use crate::app::state::{AppState, Focus, View};
use crate::history::HistoryService;
use crate::media::search::SearchState;
use crate::ui::icons::{Icons, sanitize_terminal_text};
use crate::ui::theme::Theme;
use crate::ui::widgets::{format_time, spinner};

/// Dispatch to the renderer for the active view.
pub fn render_main(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    history: Option<&HistoryService>,
    icons: &Icons,
    theme: &Theme,
) {
    state.main_area = area;
    match state.view {
        View::Home => render_home(frame, area, state, history, icons, theme),
        View::Search => render_search(frame, area, state, icons, theme),
        View::Queue => render_queue(frame, area, state, theme),
        View::Playlists => render_playlists(frame, area, state, theme),
        View::PlaylistDetail => render_playlist_detail(frame, area, state, theme),
        View::History => render_history(frame, area, state, history, theme),
        View::NowPlaying => render_now_playing_view(frame, area, state, icons, theme),
        View::Help => render_help(frame, area, theme),
    }
}

/// Standard rounded block for a view, with a styled title.
fn view_block<'a>(title: Line<'a>, theme: &Theme) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title(title)
}

/// Render a vertical scrollbar for a list inside a bordered area.
fn render_scrollbar(frame: &mut Frame, area: Rect, content_len: usize, position: usize) {
    let mut scrollbar_state = ScrollbarState::new(content_len).position(position);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼")),
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

/// Home dashboard: resume card on top, recent tracks and playlists below.
fn render_home(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    history: Option<&HistoryService>,
    icons: &Icons,
    theme: &Theme,
) {
    use crate::app::state::HomeSection;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(3)])
        .split(area);

    // --- Resume card ----------------------------------------------------
    let focused = state.home_section;
    let section_block = |title: &str, section: HomeSection| {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(if focused == section {
                theme.border_active
            } else {
                theme.border
            })
            .title(Span::styled(
                format!(" {title} "),
                if focused == section {
                    theme.accent
                } else {
                    theme.dim
                },
            ))
    };

    let resume_block = section_block("Resume", HomeSection::Resume);
    let resume_inner = resume_block.inner(rows[0]);
    frame.render_widget(resume_block, rows[0]);

    let resume_lines: Vec<Line> = if let Some(pending) = &state.pending_resume {
        let width = resume_inner.width as usize;
        let title_line = Line::from(vec![
            Span::styled(format!("{} ", icons.paused), theme.warning),
            Span::styled(
                crate::ui::widgets::truncate_end(
                    &sanitize_terminal_text(&pending.track.artist),
                    (width / 3).max(10),
                ),
                theme.accent,
            ),
            Span::styled(" — ", theme.dim),
            Span::styled(
                crate::ui::widgets::truncate_middle(
                    &sanitize_terminal_text(&pending.track.title),
                    (width * 2 / 3).max(10),
                ),
                theme.header,
            ),
        ]);
        let position = format_time(pending.position_seconds);
        let total = pending
            .track
            .duration_seconds
            .map(|d| format_time(d as f64))
            .unwrap_or_else(|| "--:--".to_string());
        let status = if pending.armed {
            Span::styled("Space resume", theme.playing)
        } else if pending.play_on_load {
            Span::styled(
                format!("{} starting as soon as it's ready...", spinner(state.spinner_frame)),
                theme.accent,
            )
        } else {
            Span::styled(
                format!("{} resolving... (Space plays when ready)", spinner(state.spinner_frame)),
                theme.dim,
            )
        };
        vec![
            title_line,
            Line::from(vec![
                Span::styled(format!("at {position} / {total}   "), theme.base),
                status,
            ]),
        ]
    } else if let Some(track) = &state.current_track {
        let width = resume_inner.width as usize;
        let playing = state.playback.status == crate::playback::PlaybackStatus::Playing;
        vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", if playing { icons.playing } else { icons.paused }),
                    if playing { theme.playing } else { theme.warning },
                ),
                Span::styled(
                    crate::ui::widgets::truncate_end(
                        &sanitize_terminal_text(&track.artist),
                        (width / 3).max(10),
                    ),
                    theme.accent,
                ),
                Span::styled(" — ", theme.dim),
                Span::styled(
                    crate::ui::widgets::truncate_middle(
                        &sanitize_terminal_text(&track.title),
                        (width * 2 / 3).max(10),
                    ),
                    theme.header,
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "{} / {}   Space {}   6 open Playing",
                    format_time(state.playback.position_seconds),
                    state
                        .playback
                        .duration_seconds
                        .map(format_time)
                        .unwrap_or_else(|| "--:--".to_string()),
                    if playing { "pause" } else { "resume" },
                ),
                theme.dim,
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled("Nothing to resume yet", theme.dim)),
            Line::from(Span::styled(
                "Press / to search, or pick something below",
                theme.dim,
            )),
        ]
    };
    frame.render_widget(Paragraph::new(resume_lines), resume_inner);

    // --- Recent and Playlists panes -------------------------------------
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(rows[1]);

    let recent_cap = (panes[0].height.saturating_sub(2) as usize).min(30);
    let recent = history
        .map(|h| h.recent_unique(recent_cap.max(1)))
        .unwrap_or_default();
    state.home_recent_len = recent.len();

    let recent_block = section_block("Recent", HomeSection::Recent);
    let recent_inner = recent_block.inner(panes[0]);
    frame.render_widget(recent_block, panes[0]);
    if recent.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No listening history yet", theme.dim)),
            recent_inner,
        );
    } else {
        let width = recent_inner.width as usize;
        let items: Vec<ListItem> = recent
            .iter()
            .map(|t| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        crate::ui::widgets::truncate_end(
                            &format!(
                                "{} — {}",
                                sanitize_terminal_text(&t.artist),
                                sanitize_terminal_text(&t.title)
                            ),
                            width.saturating_sub(2).max(8),
                        ),
                        theme.base,
                    ),
                ]))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(theme.selected)
            .highlight_symbol("▶ ");
        let mut list_state = ratatui::widgets::ListState::default();
        if focused == HomeSection::Recent {
            list_state.select(Some(state.selected_index));
        }
        frame.render_stateful_widget(list, recent_inner, &mut list_state);
    }

    let playlists_block = section_block("Playlists", HomeSection::Playlists);
    let playlists_inner = playlists_block.inner(panes[1]);
    frame.render_widget(playlists_block, panes[1]);
    if state.playlists.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No playlists yet — N to create", theme.dim)),
            playlists_inner,
        );
    } else {
        let items: Vec<ListItem> = state
            .playlists
            .iter()
            .map(|p| {
                let imported = if p.source.is_some() { " ↓" } else { "" };
                ListItem::new(Line::from(vec![
                    Span::raw(sanitize_terminal_text(&p.name)),
                    Span::styled(format!(" ({}){imported}", p.tracks.len()), theme.dim),
                ]))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(theme.selected)
            .highlight_symbol("▶ ");
        let mut list_state = ratatui::widgets::ListState::default();
        if focused == HomeSection::Playlists {
            list_state.select(Some(state.selected_index));
        }
        frame.render_stateful_widget(list, playlists_inner, &mut list_state);
    }
}

fn render_search(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Search input box.
    let input_focused = state.focus == Focus::SearchInput;
    let input_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(if input_focused {
            theme.border_active
        } else {
            theme.border
        })
        .title(Span::styled(
            format!(" {} Search ", icons.search),
            if input_focused {
                theme.accent
            } else {
                theme.dim
            },
        ));
    let input_inner = input_block.inner(chunks[0]);
    frame.render_widget(input_block, chunks[0]);

    let input_line = if state.search_input.is_empty() && !input_focused {
        Line::from(Span::styled(
            "Press / to search, or paste a YouTube URL...",
            theme.dim,
        ))
    } else {
        Line::from(vec![
            Span::raw(sanitize_terminal_text(&state.search_input)),
            if input_focused {
                Span::styled("▏", theme.accent)
            } else {
                Span::raw("")
            },
        ])
    };
    frame.render_widget(Paragraph::new(input_line), input_inner);

    // Results area.
    match &state.search {
        SearchState::Idle => {
            let block = view_block(Line::from(" Results "), theme);
            let inner = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("{} Type a query and press Enter", icons.search),
                        theme.dim,
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Tip: pasting a playlist URL starts an import",
                        theme.dim,
                    )),
                ]),
                inner,
            );
        }
        SearchState::Searching { query, .. } => {
            let block = view_block(Line::from(" Results "), theme);
            let inner = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(format!("{} ", spinner(state.spinner_frame)), theme.accent),
                        Span::styled(
                            format!("Searching for \"{}\"...", sanitize_terminal_text(query)),
                            theme.base,
                        ),
                    ]),
                ]),
                inner,
            );
        }
        SearchState::Failed { message, .. } => {
            let block = view_block(Line::from(" Results "), theme);
            let inner = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("{} {}", icons.error, sanitize_terminal_text(message)),
                        theme.error,
                    )),
                ]),
                inner,
            );
        }
        SearchState::Results { tracks, .. } => {
            let count = tracks.len();
            let title = if count == 0 {
                " Results ".to_string()
            } else {
                format!(" Results ({count}) ")
            };
            let block = view_block(Line::from(title), theme);
            let inner = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);

            if count == 0 {
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled("No results — try another query", theme.dim)),
                    ]),
                    inner,
                );
                return;
            }

            // Approximate the column widths ratatui will compute so cells
            // can ellipsize instead of hard-cutting mid-word. The "▶ "
            // highlight symbol and column spacing eat into the total.
            let usable = (inner.width as usize).saturating_sub(2 + 2 + 7);
            let title_width = (usable * 55 / 90).max(8);
            let artist_width = usable.saturating_sub(title_width).max(8);
            let rows: Vec<Row> = tracks
                .iter()
                .map(|t| {
                    let duration = t
                        .duration_seconds
                        .map(|d| format_time(d as f64))
                        .unwrap_or_else(|| "--:--".to_string());
                    Row::new(vec![
                        crate::ui::widgets::truncate_middle(
                            &sanitize_terminal_text(&t.title),
                            title_width,
                        ),
                        crate::ui::widgets::truncate_end(
                            &sanitize_terminal_text(&t.artist),
                            artist_width,
                        ),
                        duration,
                    ])
                })
                .collect();
            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(55),
                    Constraint::Percentage(35),
                    Constraint::Length(7),
                ],
            )
            .header(
                Row::new(vec!["Title", "Artist", "Time"])
                    .style(theme.dim)
                    .bottom_margin(1),
            )
            .row_highlight_style(theme.selected)
            .highlight_symbol("▶ ");
            state.table_state.select(Some(state.selected_index));
            frame.render_stateful_widget(table, inner, &mut state.table_state);
            render_scrollbar(frame, chunks[1], count, state.table_state.offset());
        }
    }
}

fn render_queue(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let total = state.queue.order.len();
    let modes = {
        let mut parts = Vec::new();
        if state.queue.shuffle {
            parts.push("shuffle");
        }
        match state.queue.repeat {
            crate::queue::RepeatMode::Track => parts.push("repeat track"),
            crate::queue::RepeatMode::Queue => parts.push("repeat queue"),
            crate::queue::RepeatMode::Off => {}
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" • {}", parts.join(" • "))
        }
    };
    let block = view_block(Line::from(format!(" Queue ({total}){modes} ")), theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if total == 0 {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("Queue is empty", theme.dim)),
                Line::from(""),
                Line::from(Span::styled(
                    "Search for something and press a to add it",
                    theme.dim,
                )),
            ]),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .queue
        .order
        .iter()
        .enumerate()
        .map(|(pos, &i)| {
            let track = &state.queue.tracks[i];
            let is_current = Some(pos) == state.queue.position;
            let marker = if is_current { "♪" } else { " " };
            let duration = track
                .duration_seconds
                .map(|d| format_time(d as f64))
                .unwrap_or_else(|| "--:--".to_string());
            let style = if is_current { theme.accent } else { theme.base };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{marker} "),
                    if is_current { theme.playing } else { theme.dim },
                ),
                Span::styled(
                    format!(
                        "{} — {}",
                        sanitize_terminal_text(&track.artist),
                        sanitize_terminal_text(&track.title)
                    ),
                    style,
                ),
                Span::styled(format!("  {duration}"), theme.dim),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(theme.selected)
        .highlight_symbol("▶ ");
    state.list_state.select(Some(state.selected_index));
    frame.render_stateful_widget(list, inner, &mut state.list_state);
    render_scrollbar(frame, area, total, state.list_state.offset());
}

fn render_playlists(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let block = view_block(
        Line::from(format!(" Playlists ({}) ", state.playlists.len())),
        theme,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.playlists.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("No playlists yet", theme.dim)),
                Line::from(""),
                Line::from(Span::styled(
                    "i import from YouTube   N new playlist",
                    theme.dim,
                )),
                Line::from(Span::styled(
                    "w save the current queue (in Queue view)",
                    theme.dim,
                )),
            ]),
            inner,
        );
        return;
    }

    // Two panes: playlist list on the left, preview of the selected
    // playlist's tracks on the right.
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(inner);

    let items: Vec<ListItem> = state
        .playlists
        .iter()
        .map(|p| {
            let imported = if p.source.is_some() { " ↓" } else { "" };
            ListItem::new(Line::from(vec![
                Span::raw(sanitize_terminal_text(&p.name)),
                Span::styled(format!(" ({}){imported}", p.tracks.len()), theme.dim),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(theme.selected)
        .highlight_symbol("▶ ");
    state.list_state.select(Some(state.selected_index));
    frame.render_stateful_widget(list, panes[0], &mut state.list_state);

    // Preview pane.
    let preview_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title(Span::styled(" Preview ", theme.dim));
    let preview_inner = preview_block.inner(panes[1]);
    frame.render_widget(preview_block, panes[1]);

    if let Some(playlist) = state.playlists.get(state.selected_index) {
        let mut lines: Vec<Line> = playlist
            .tracks
            .iter()
            .take(12)
            .map(|t| {
                Line::from(Span::styled(
                    format!(
                        "{} — {}",
                        sanitize_terminal_text(&t.artist),
                        sanitize_terminal_text(&t.title)
                    ),
                    theme.base,
                ))
            })
            .collect();
        if playlist.tracks.len() > 12 {
            lines.push(Line::from(Span::styled(
                format!("... and {} more", playlist.tracks.len() - 12),
                theme.dim,
            )));
        }
        if let Some(source) = &playlist.source {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("from: {}", sanitize_terminal_text(&source.url)),
                theme.dim,
            )));
        }
        frame.render_widget(Paragraph::new(lines), preview_inner);
    }
    render_scrollbar(
        frame,
        area,
        state.playlists.len(),
        state.list_state.offset(),
    );
}

fn render_playlist_detail(frame: &mut Frame, area: Rect, state: &mut AppState, theme: &Theme) {
    let Some(playlist) = state.selected_playlist.and_then(|i| state.playlists.get(i)) else {
        let block = view_block(Line::from(" Playlist "), theme);
        frame.render_widget(block, area);
        return;
    };
    let title = format!(
        " {} ({}) ",
        sanitize_terminal_text(&playlist.name),
        playlist.tracks.len()
    );
    let block = view_block(Line::from(title), theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if playlist.tracks.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("Playlist is empty", theme.dim)),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = playlist
        .tracks
        .iter()
        .map(|t| {
            let duration = t
                .duration_seconds
                .map(|d| format_time(d as f64))
                .unwrap_or_else(|| "--:--".to_string());
            ListItem::new(Line::from(vec![
                Span::raw(format!(
                    "{} — {}",
                    sanitize_terminal_text(&t.artist),
                    sanitize_terminal_text(&t.title)
                )),
                Span::styled(format!("  {duration}"), theme.dim),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(theme.selected)
        .highlight_symbol("▶ ");
    state.list_state.select(Some(state.selected_index));
    frame.render_stateful_widget(list, inner, &mut state.list_state);
    render_scrollbar(
        frame,
        area,
        playlist.tracks.len(),
        state.list_state.offset(),
    );
}

fn render_history(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    history: Option<&HistoryService>,
    theme: &Theme,
) {
    let entries = history.map(|h| h.entries()).unwrap_or(&[]);
    let block = view_block(Line::from(format!(" History ({}) ", entries.len())), theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No playback history", theme.dim)),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .map(|e| {
            let outcome_style = match e.outcome {
                crate::history::model::PlaybackOutcome::Completed => theme.playing,
                crate::history::model::PlaybackOutcome::Skipped => theme.dim,
                crate::history::model::PlaybackOutcome::Failed => theme.error,
                crate::history::model::PlaybackOutcome::Stopped => theme.warning,
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:?}", e.outcome), outcome_style),
                Span::styled("  ", theme.dim),
                Span::raw(format!(
                    "{} — {}",
                    sanitize_terminal_text(&e.artist),
                    sanitize_terminal_text(&e.title)
                )),
                Span::styled(
                    format!("  {}", e.played_at.format("%Y-%m-%d %H:%M")),
                    theme.dim,
                ),
            ]))
        })
        .collect();
    let count = items.len();
    let list = List::new(items)
        .highlight_style(theme.selected)
        .highlight_symbol("▶ ");
    state.list_state.select(Some(state.selected_index));
    frame.render_stateful_widget(list, inner, &mut state.list_state);
    render_scrollbar(frame, area, count, state.list_state.offset());
}

fn render_now_playing_view(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let block = view_block(Line::from(" Now Playing "), theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(track) = state.current_track.clone() else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("Nothing is playing", theme.dim)),
                Line::from(""),
                Line::from(Span::styled(
                    "Search (2) and press Enter on a result",
                    theme.dim,
                )),
            ]),
            inner,
        );
        return;
    };

    // --- Hero: artwork cell + track info --------------------------------
    // A fixed-size artwork cell instead of a half-screen pane: thumbnails
    // are small 16:9 images, a big box just letterboxes them in border.
    let narrow = inner.width < 90;
    let hero_height = 8u16.min(inner.height);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(hero_height), Constraint::Min(0)])
        .split(inner);

    let art_width = if narrow { 0 } else { 26 };
    let hero = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(art_width), Constraint::Min(20)])
        .split(rows[0]);

    if art_width > 0 {
        let art_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.border);
        let art_inner = art_block.inner(hero[0]);
        frame.render_widget(art_block, hero[0]);
        if let Some(protocol) = state.thumbnail.as_mut() {
            let image_widget =
                ratatui_image::StatefulImage::<ratatui_image::protocol::StatefulProtocol>::new();
            frame.render_stateful_widget(image_widget, art_inner, protocol);
        } else {
            let placeholder = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(icons.music, theme.dim)).centered(),
            ]);
            frame.render_widget(placeholder, art_inner);
        }
    }

    let info_area = hero[1].inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    let (status_icon, status_style) = match state.playback.status {
        crate::playback::PlaybackStatus::Playing => (icons.playing, theme.playing),
        crate::playback::PlaybackStatus::Paused => (icons.paused, theme.warning),
        _ => (icons.stopped, theme.dim),
    };
    let position = state.playback.position_seconds;
    let duration = state.playback.duration_seconds.unwrap_or(0.0);
    let info_width = info_area.width as usize;
    let title_width = info_width.saturating_sub(status_icon.chars().count() + 1);

    let mut info_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(format!("{status_icon} "), status_style),
            Span::styled(
                crate::ui::widgets::truncate_middle(
                    &sanitize_terminal_text(&track.title),
                    title_width.max(8),
                ),
                theme.header,
            ),
        ]),
        Line::from(Span::styled(
            crate::ui::widgets::truncate_end(
                &sanitize_terminal_text(&track.artist),
                info_width.max(8),
            ),
            theme.accent,
        )),
    ];

    // Stats from extended metadata (fetched in the background).
    if let Some(details) = &state.current_details {
        let mut stats: Vec<Span> = Vec::new();
        if let Some(views) = details.view_count {
            stats.push(Span::styled(
                format!("{} views", crate::media::format_count(views)),
                theme.base,
            ));
        }
        if let Some(likes) = details.like_count {
            if !stats.is_empty() {
                stats.push(Span::styled("  \u{2022}  ", theme.dim));
            }
            stats.push(Span::styled(
                format!("{} likes", crate::media::format_count(likes)),
                theme.base,
            ));
        }
        if let Some(date) = details.formatted_upload_date() {
            if !stats.is_empty() {
                stats.push(Span::styled("  \u{2022}  ", theme.dim));
            }
            stats.push(Span::styled(date, theme.base));
        }
        if !details.categories.is_empty() {
            if !stats.is_empty() {
                stats.push(Span::styled("  \u{2022}  ", theme.dim));
            }
            stats.push(Span::styled(
                details
                    .categories
                    .iter()
                    .map(|c| sanitize_terminal_text(c))
                    .collect::<Vec<_>>()
                    .join(" "),
                theme.accent_alt,
            ));
        }
        info_lines.push(Line::from(stats));
    } else {
        info_lines.push(Line::from(Span::styled(
            format!("{} Loading details...", spinner(state.spinner_frame)),
            theme.dim,
        )));
    }
    info_lines.push(Line::from(""));
    frame.render_widget(Paragraph::new(info_lines), info_area);

    // Progress gauge with inline times, right under the metadata.
    if info_area.height >= 6 {
        let gauge_area = Rect {
            x: info_area.x,
            y: info_area.y + 4,
            width: info_area.width,
            height: 1,
        };
        let ratio = if duration > 0.0 {
            (position / duration).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let gauge = ratatui::widgets::LineGauge::default()
            .filled_style(theme.gauge_filled)
            .filled_symbol(ratatui::symbols::line::THICK.horizontal)
            .style(theme.border)
            .ratio(ratio)
            .label(Span::styled(
                format!(
                    "{} / {}",
                    format_time(position),
                    state
                        .playback
                        .duration_seconds
                        .map(format_time)
                        .unwrap_or_else(|| "--:--".to_string())
                ),
                theme.base,
            ));
        frame.render_widget(gauge, gauge_area);

        // Status line: volume, modes, queue position.
        let mut status_parts = vec![format!(
            "vol {}%{}",
            state.playback.volume,
            if state.playback.muted { " (muted)" } else { "" }
        )];
        if state.queue.shuffle {
            status_parts.push("shuffle".to_string());
        }
        match state.queue.repeat {
            crate::queue::RepeatMode::Track => status_parts.push("repeat track".to_string()),
            crate::queue::RepeatMode::Queue => status_parts.push("repeat queue".to_string()),
            crate::queue::RepeatMode::Off => {}
        }
        if let Some(pos) = state.queue.position {
            status_parts.push(format!("queue {}/{}", pos + 1, state.queue.order.len()));
        }
        let status_area = Rect {
            x: info_area.x,
            y: info_area.y + 5,
            width: info_area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                status_parts.join("  \u{2022}  "),
                theme.dim,
            )),
            status_area,
        );
    }

    if rows[1].height < 3 {
        return;
    }

    // --- Bottom: Up Next + chapters/description -------------------------
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if narrow {
            [Constraint::Percentage(0), Constraint::Percentage(100)]
        } else {
            [Constraint::Percentage(38), Constraint::Percentage(62)]
        })
        .split(rows[1]);

    if !narrow {
        render_up_next(frame, bottom[0], state, theme);
    }

    let chapters = state.chapters().to_vec();
    let show_chapters = !chapters.is_empty() && !state.now_playing_show_description;
    if show_chapters {
        render_chapters(frame, bottom[1], state, &chapters, theme);
    } else {
        render_description(frame, bottom[1], state, !chapters.is_empty(), theme);
    }
}

/// A window of the queue after the current track.
fn render_up_next(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title(Span::styled(" Up Next ", theme.dim));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(position) = state.queue.position else {
        frame.render_widget(
            Paragraph::new(Span::styled("Queue is empty", theme.dim)),
            inner,
        );
        return;
    };
    let upcoming: Vec<Line> = state
        .queue
        .order
        .iter()
        .enumerate()
        .skip(position + 1)
        .take(inner.height as usize)
        .map(|(pos, &i)| {
            let track = &state.queue.tracks[i];
            Line::from(vec![
                Span::styled(format!("{:>3}  ", pos + 1), theme.dim),
                Span::styled(
                    crate::ui::widgets::truncate_end(
                        &format!(
                            "{} — {}",
                            sanitize_terminal_text(&track.artist),
                            sanitize_terminal_text(&track.title)
                        ),
                        (inner.width as usize).saturating_sub(5).max(8),
                    ),
                    theme.base,
                ),
            ])
        })
        .collect();
    if upcoming.is_empty() {
        let hint = match state.queue.repeat {
            crate::queue::RepeatMode::Queue => "Queue repeats from the top",
            _ => "Queue ends after this track",
        };
        frame.render_widget(Paragraph::new(Span::styled(hint, theme.dim)), inner);
    } else {
        frame.render_widget(Paragraph::new(upcoming), inner);
    }
}

/// Chapter list with the playing chapter highlighted and kept in view.
fn render_chapters(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    chapters: &[crate::media::Chapter],
    theme: &Theme,
) {
    let current = state.current_chapter_index();
    let title = format!(
        " Chapters ({}/{}) — ./, jump · v description ",
        current.map(|i| i + 1).unwrap_or(0),
        chapters.len()
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title(Span::styled(title, theme.dim));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Keep the current chapter vertically centered.
    let visible = inner.height as usize;
    let offset = current
        .unwrap_or(0)
        .saturating_sub(visible.saturating_sub(1) / 2)
        .min(chapters.len().saturating_sub(visible));

    let width = inner.width as usize;
    let lines: Vec<Line> = chapters
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, chapter)| {
            let is_current = Some(i) == current;
            let marker = if is_current { "▶" } else { " " };
            let timestamp = format_time(chapter.start_seconds);
            Line::from(vec![
                Span::styled(
                    format!("{marker} {timestamp:>7}  "),
                    if is_current { theme.playing } else { theme.dim },
                ),
                Span::styled(
                    crate::ui::widgets::truncate_end(
                        &sanitize_terminal_text(&chapter.title),
                        width.saturating_sub(11).max(8),
                    ),
                    if is_current { theme.accent } else { theme.base },
                ),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
    if chapters.len() > visible {
        render_scrollbar(frame, area, chapters.len(), offset);
    }
}

/// Scrollable description panel (line breaks preserved).
fn render_description(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    has_chapters: bool,
    theme: &Theme,
) {
    let title = if has_chapters {
        " Description (j/k scroll · v chapters) "
    } else {
        " Description (j/k scroll) "
    };
    let desc_block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border)
        .title(Span::styled(title, theme.dim));
    let desc_inner = desc_block.inner(area);
    frame.render_widget(desc_block, area);

    let description = state
        .current_details
        .as_ref()
        .and_then(|d| d.description.as_deref())
        .unwrap_or("No description available.");
    // Preserve line breaks (tracklists!) and measure the *sanitized* text so
    // the scrollbar length matches what is actually rendered.
    let description = crate::ui::icons::sanitize_multiline_text(description);
    let wrap_width = desc_inner.width.max(1) as usize;
    let line_count = description
        .lines()
        .map(|l| l.chars().count().max(1).div_ceil(wrap_width))
        .sum::<usize>();
    let paragraph = Paragraph::new(description)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .style(theme.dim)
        .scroll((state.now_playing_scroll, 0));
    frame.render_widget(paragraph, desc_inner);
    if line_count > desc_inner.height as usize {
        render_scrollbar(frame, area, line_count, state.now_playing_scroll as usize);
    }
}

fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let block = view_block(Line::from(" Help "), theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let section =
        |title: &str| Row::new(vec![title.to_string(), String::new()]).style(theme.accent);
    let binding = |key: &str, desc: &str| {
        Row::new(vec![format!("  {key:<10}"), desc.to_string()]).style(theme.base)
    };
    let rows = vec![
        section("Views"),
        binding("1", "Home"),
        binding("2", "Search"),
        binding("3", "Queue"),
        binding("4", "Playlists"),
        binding("5", "History"),
        binding("6", "Now Playing"),
        section("Playback"),
        binding("Space", "Play / pause"),
        binding("n / b", "Next / previous track"),
        binding("h / l", "Seek 5 seconds"),
        binding("H / L", "Seek 30 seconds"),
        binding("+ / -", "Volume"),
        binding("m", "Mute"),
        binding("s", "Shuffle"),
        binding("r", "Repeat mode"),
        section("Lists"),
        binding("j / k", "Move selection"),
        binding("Enter", "Play / open"),
        binding("a / A", "Add to queue / play next"),
        binding("J / K", "Move queue item down / up"),
        section("Playlists"),
        binding("i", "Import from URL"),
        binding("N", "New playlist"),
        binding("R", "Rename"),
        binding("x", "Delete (asks first)"),
        binding("w", "Save queue as playlist"),
        section("Other"),
        binding("/", "Focus search"),
        binding("?", "This help"),
        binding("q", "Quit"),
    ];
    let table = Table::new(rows, [Constraint::Length(14), Constraint::Min(30)]).column_spacing(2);
    frame.render_widget(table, inner);
}
