//! Responsive Home dashboard renderer.

use super::*;
use crate::app::state::HomeSection;
use crate::ui::layout::Breakpoint;
use crate::ui::views::home_panels::{render_playlists, render_recent};
use crate::ui::views::home_resume::render_resume;

struct DashboardData<'a> {
    history: Option<&'a HistoryService>,
    recent: &'a [crate::media::Track],
    icons: &'a Icons,
    theme: &'a Theme,
    show_resume: bool,
}

pub(super) fn render_home(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    history: Option<&HistoryService>,
    icons: &Icons,
    theme: &Theme,
) {
    let recent = history
        .map(|service| service.recent_unique((area.height as usize).clamp(4, 30)))
        .unwrap_or_default();
    let show_resume = actionable_resume(state);
    state.ui.home_recent_len = recent.len();
    if !show_resume && recent.is_empty() && state.domain.playlists.is_empty() {
        render_first_run(frame, area, theme);
        return;
    }
    let data = DashboardData {
        history,
        recent: &recent,
        icons,
        theme,
        show_resume,
    };
    if !show_resume && state.ui.home_section == HomeSection::Resume {
        state.ui.home_section = if !recent.is_empty() {
            HomeSection::Recent
        } else if !state.domain.playlists.is_empty() {
            HomeSection::Playlists
        } else {
            HomeSection::Recent
        };
    }
    match Breakpoint::from_width(area.width) {
        Breakpoint::Narrow => render_narrow(frame, area, state, &data),
        Breakpoint::Medium => render_medium(frame, area, state, &data),
        Breakpoint::Wide | Breakpoint::UltraWide => render_wide(frame, area, state, &data),
    }
}

fn render_first_run(frame: &mut Frame, area: Rect, theme: &Theme) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Length(5),
            Constraint::Percentage(65),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("Welcome to ytm", theme.header)).centered(),
            Line::from("").centered(),
            Line::from(Span::styled(
                "Press / to search · 4 for playlists · ? for help",
                theme.accent,
            ))
            .centered(),
        ]),
        rows[1],
    );
}

fn render_narrow(frame: &mut Frame, area: Rect, state: &mut AppState, data: &DashboardData<'_>) {
    let count = usize::from(data.show_resume)
        + usize::from(!data.recent.is_empty())
        + usize::from(!state.domain.playlists.is_empty());
    let constraints = (0..count)
        .map(|_| Constraint::Ratio(1, count as u32))
        .collect::<Vec<_>>();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .spacing(crate::ui::layout::PANE_GUTTER)
        .split(area);
    let mut row = 0;
    if data.show_resume {
        render_resume(
            frame,
            rows[row],
            state,
            data.history,
            data.icons,
            data.theme,
        );
        row += 1;
    }
    if !data.recent.is_empty() {
        render_recent(
            frame,
            rows[row],
            state,
            data.recent,
            data.history.map_or(0, |service| service.entries().len()),
            data.icons,
            data.theme,
        );
        row += 1;
    }
    if !state.domain.playlists.is_empty() {
        render_playlists(frame, rows[row], state, data.icons, data.theme);
    }
}

fn render_medium(frame: &mut Frame, area: Rect, state: &mut AppState, data: &DashboardData<'_>) {
    if !data.show_resume {
        render_without_resume(frame, area, state, data);
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .spacing(crate::ui::layout::PANE_GUTTER)
        .split(area);
    let top = halves(rows[0]);
    render_resume(frame, top[0], state, data.history, data.icons, data.theme);
    render_recent(
        frame,
        top[1],
        state,
        data.recent,
        data.history.map_or(0, |service| service.entries().len()),
        data.icons,
        data.theme,
    );
    render_playlists(frame, rows[1], state, data.icons, data.theme);
}

fn render_wide(frame: &mut Frame, area: Rect, state: &mut AppState, data: &DashboardData<'_>) {
    if !data.show_resume {
        render_without_resume(frame, area, state, data);
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .spacing(crate::ui::layout::PANE_GUTTER)
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .spacing(crate::ui::layout::PANE_GUTTER)
        .split(rows[0]);
    render_resume(frame, top[0], state, data.history, data.icons, data.theme);
    render_recent(
        frame,
        top[1],
        state,
        data.recent,
        data.history.map_or(0, |service| service.entries().len()),
        data.icons,
        data.theme,
    );
    render_playlists(frame, rows[1], state, data.icons, data.theme);
}

fn render_without_resume(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    data: &DashboardData<'_>,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .spacing(crate::ui::layout::PANE_GUTTER)
        .split(area);
    render_recent(
        frame,
        rows[0],
        state,
        data.recent,
        data.history.map_or(0, |service| service.entries().len()),
        data.icons,
        data.theme,
    );
    render_playlists(frame, rows[1], state, data.icons, data.theme);
}

fn actionable_resume(state: &AppState) -> bool {
    let Some(pending) = &state.domain.pending_resume else {
        return false;
    };
    if state.domain.playback.status == crate::playback::PlaybackStatus::Playing {
        return false;
    }
    state
        .domain
        .current_track
        .as_ref()
        .is_none_or(|current| current.id == pending.track.id)
}

fn halves(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(crate::ui::layout::PANE_GUTTER)
        .split(area)
}
