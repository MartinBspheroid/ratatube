//! Mouse selection, scrolling, tab activation, and progress seeking.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction, PlaybackAction};
use crate::app::state::View;

impl App {
    /// Mouse input: wheel scrolls the list, click selects, double-click plays.
    pub(super) async fn handle_mouse(
        &mut self,
        mouse: crossterm::event::MouseEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        use crossterm::event::{MouseButton, MouseEventKind};
        if self.state.prompt.is_some()
            || self.state.confirm.is_some()
            || self.state.import.is_some()
            || self.state.picker.is_some()
            || self.state.search_detail_open
        {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if self.state.view == View::NowPlaying {
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::ScrollNowPlaying(3)))
                        .await;
                } else {
                    for _ in 0..3 {
                        let _ = action_tx
                            .send(Action::Navigation(NavigationAction::SelectNext))
                            .await;
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.state.view == View::NowPlaying {
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::ScrollNowPlaying(-3)))
                        .await;
                } else {
                    for _ in 0..3 {
                        let _ = action_tx
                            .send(Action::Navigation(NavigationAction::SelectPrevious))
                            .await;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.row == 0 {
                    let icons = crate::ui::icons::icons_for(self.state.icon_mode);
                    let narrow =
                        crate::ui::layout::Breakpoint::from_width(self.state.screen_area.width)
                            == crate::ui::layout::Breakpoint::Narrow;
                    for (view, start, end) in
                        crate::ui::widgets::tab_hit_zones(&icons, self.state.view, narrow)
                    {
                        if mouse.column >= start && mouse.column < end {
                            let _ = action_tx
                                .send(Action::Navigation(NavigationAction::Navigate(view)))
                                .await;
                            return;
                        }
                    }
                    return;
                }
                if self.state.has_now_playing() {
                    let layout =
                        crate::ui::layout::AppLayout::new(self.state.screen_area, true, true);
                    let bar = layout.now_playing;
                    let gauge_row = bar.y + 2;
                    if bar.height >= 5
                        && mouse.row == gauge_row
                        && mouse.column > bar.x
                        && mouse.column < bar.x + bar.width.saturating_sub(1)
                    {
                        let inner_width = bar.width.saturating_sub(2);
                        let fraction =
                            f64::from(mouse.column - bar.x - 1) / f64::from(inner_width.max(1));
                        let _ = action_tx
                            .send(Action::Playback(PlaybackAction::SeekToFraction(fraction)))
                            .await;
                        return;
                    }
                }
                let area = self.state.list_hit_area;
                if mouse.column >= area.x
                    && mouse.column < area.x + area.width
                    && mouse.row >= area.y
                    && mouse.row < area.y + area.height
                {
                    let offset = if self.state.view == View::Search {
                        self.state.table_state.offset()
                    } else {
                        self.state.list_state.offset()
                    };
                    let index = offset + usize::from(mouse.row - area.y);
                    if index < self.state.active_list_len() {
                        self.state.selected_index = index;
                        let now = Instant::now();
                        let target = (self.state.view, index);
                        let double_click = self.last_click.is_some_and(|(at, view, clicked)| {
                            (view, clicked) == target
                                && now.duration_since(at) < Duration::from_millis(400)
                        });
                        self.last_click = Some((now, target.0, target.1));
                        if double_click {
                            let _ = action_tx
                                .send(Action::Playback(PlaybackAction::PlaySelected))
                                .await;
                            self.last_click = None;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
