//! View renderers for each primary screen (PRD section 9).
//!
//! Views render from [`AppState`] and mutate only `list_state` so ratatui
//! can scroll lists naturally; all data changes flow through actions.

mod channel;
mod history;
mod home;
mod home_panels;
mod home_resume;
mod playing;
mod playing_hero;
mod playing_panels;
mod playlists;
mod queue;
mod search;
mod search_detail;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Table};

use crate::app::state::{AppState, Focus, View};
use crate::history::HistoryService;
use crate::media::search::SearchState;
use crate::ui::components::{NumberedRow, key_value_rows, numbered_row, scrollbar, section_panel};
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
    state.ui.main_area = area;
    state.ui.list_hit_area = Rect::default();
    match state.ui.view {
        View::Home => home::render_home(frame, area, state, history, icons, theme),
        View::Search => search::render_search(frame, area, state, icons, theme),
        View::Queue => queue::render_queue(frame, area, state, icons, theme),
        View::Playlists => playlists::render_playlists(frame, area, state, icons, theme),
        View::PlaylistDetail => playlists::render_playlist_detail(frame, area, state, icons, theme),
        View::Channel => channel::render_channel(frame, area, state, icons, theme),
        View::History => history::render_history(frame, area, state, history, icons, theme),
        View::NowPlaying => playing::render_now_playing_view(frame, area, state, icons, theme),
        View::Help => render_help(frame, area, state, icons, theme),
    }
}

/// Render the in-list filter bar (`/`) when active and return the area left
/// for the list itself.
fn render_filter_bar(
    frame: &mut Frame,
    inner: Rect,
    state: &AppState,
    total: usize,
    icons: &Icons,
    theme: &Theme,
) -> Rect {
    let Some(filter) = &state.ui.list_filter else {
        return inner;
    };
    if inner.height == 0 {
        return inner;
    }
    let editing = state.ui.focus == Focus::ListFilter;
    let shown = state.ui.visible_indices.as_ref().map_or(total, Vec::len);
    let line = Line::from(vec![
        Span::styled("/", theme.accent),
        Span::raw(sanitize_terminal_text(filter)),
        Span::styled(if editing { icons.section_bar } else { "" }, theme.accent),
        Span::styled(format!("   {shown} of {total}"), theme.dim),
        Span::styled(
            if editing {
                "   Enter lock · Esc clear"
            } else {
                "   Esc clear"
            },
            theme.dim,
        ),
    ]);
    frame.render_widget(Paragraph::new(line), Rect { height: 1, ..inner });
    Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    }
}

/// Visible positions of a list under the active filter (identity when no
/// filter applies).
fn visible_positions(state: &AppState, total: usize) -> Vec<usize> {
    match &state.ui.visible_indices {
        Some(indices) => indices.clone(),
        None => (0..total).collect(),
    }
}

/// Home dashboard: resume card on top, recent tracks and playlists below.
fn render_help(frame: &mut Frame, area: Rect, state: &AppState, icons: &Icons, theme: &Theme) {
    let inner = section_panel(frame, area, "Help", true, theme, icons);

    let section = |title: &str| Line::from(Span::styled(title.to_string(), theme.accent));
    let binding = |key: &str, desc: &str| {
        Line::from(vec![
            Span::styled(format!("  {key:<12}"), theme.base),
            Span::styled(desc.to_string(), theme.base),
        ])
    };
    let lines: Vec<Line> = crate::input::keymap::HELP_SECTIONS
        .iter()
        .flat_map(|(title, commands)| {
            std::iter::once(section(title)).chain(
                commands
                    .iter()
                    .map(|(key, description)| binding(key, description)),
            )
        })
        .collect();
    let max_scroll = lines.len().saturating_sub(inner.height as usize) as u16;
    let scroll = state.ui.help_scroll.min(max_scroll);
    let line_count = lines.len();
    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), inner);
    scrollbar(frame, inner, line_count, scroll as usize);
}
