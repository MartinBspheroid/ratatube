//! Responsive Playing view renderer.

use super::*;
use crate::render::components::{EmptyState, empty_state, section_panel};
use crate::render::layout::Breakpoint;
use crate::render::views::playing_hero::render_hero;
use crate::render::views::playing_panels::{render_description, render_queue, render_up_next};
use crate::state::PlayingPane;

pub fn render_now_playing_view(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let breakpoint = Breakpoint::from_width(area.width);
    let focused = breakpoint != Breakpoint::UltraWide || state.ui.playing_pane == PlayingPane::Info;
    let content = section_panel(frame, area, "Now Playing", focused, theme, icons);
    let Some(track) = state.domain.current_track.clone() else {
        empty_state(
            frame,
            content,
            EmptyState {
                icon: icons.music,
                headline: "Nothing is playing",
                hints: &[("2", "open Search"), ("Enter", "play a result")],
            },
            theme,
        );
        return;
    };

    let breakpoint = Breakpoint::from_width(content.width);
    let hero_height = if breakpoint == Breakpoint::Narrow {
        8
    } else {
        9
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(hero_height), Constraint::Min(4)])
        .split(content);
    render_hero(frame, rows[0], state, &track, breakpoint, icons, theme);
    render_lower_panels(frame, rows[1], state, breakpoint, icons, theme);
}

fn render_lower_panels(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    breakpoint: Breakpoint,
    icons: &Icons,
    theme: &Theme,
) {
    match breakpoint {
        Breakpoint::Narrow => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
                .spacing(crate::render::layout::PANE_GUTTER)
                .split(area);
            render_description(frame, rows[0], state, icons, theme);
            render_up_next(frame, rows[1], state, icons, theme);
        }
        Breakpoint::Medium | Breakpoint::Wide => {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .spacing(crate::render::layout::PANE_GUTTER)
                .split(area);
            render_description(frame, columns[0], state, icons, theme);
            render_up_next(frame, columns[1], state, icons, theme);
        }
        Breakpoint::UltraWide => {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(23),
                    Constraint::Percentage(27),
                ])
                .spacing(crate::render::layout::PANE_GUTTER)
                .split(area);
            render_description(frame, columns[0], state, icons, theme);
            render_up_next(frame, columns[1], state, icons, theme);
            render_queue(frame, columns[2], state, icons, theme);
        }
    }
}
