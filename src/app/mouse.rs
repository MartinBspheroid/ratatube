//! Mouse selection, scrolling, tab activation, and progress seeking.
//!
//! Clicks are pane-aware: Home items carry per-item hit zones, the Playing
//! queue pane focuses itself, and a double-click performs exactly the action
//! Enter would (it synthesizes the key through the normal keyboard path).

use crate::app::actions::UiMsg;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaybackAction};
use crate::app::state::{AppState, Focus, HomeSection, PlayingPane, View};

/// Two clicks on the same item within this window act as Enter. Matches
/// common desktop double-click speeds; the previous 400ms missed slower
/// double-clicks.
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(500);

/// Identity of a clicked item: view, pane fingerprint, item index.
pub(super) type ClickTarget = (View, u8, usize);

/// Discriminate panes that share a view so double-clicks never span two
/// panes (Home sections, the Playing info/queue panes).
fn pane_fingerprint(state: &AppState) -> u8 {
    match state.ui.view {
        View::Home => match state.ui.home_section {
            HomeSection::Resume => 1,
            HomeSection::Recent => 2,
            HomeSection::Playlists => 3,
        },
        View::NowPlaying => match state.ui.playing_pane {
            PlayingPane::Info => 1,
            PlayingPane::Queue => 2,
        },
        _ => 0,
    }
}

impl App {
    /// Mouse input: wheel scrolls the list, click selects, double-click acts
    /// as Enter.
    pub(super) async fn handle_mouse(
        &mut self,
        mouse: crossterm::event::MouseEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        use crossterm::event::{MouseButton, MouseEventKind};
        if self.state.modal_capture().is_some() {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if self.state.ui.view == View::NowPlaying {
                    let _ = action_tx.send(Action::Ui(UiMsg::ScrollNowPlaying(3))).await;
                } else {
                    for _ in 0..3 {
                        let _ = action_tx.send(Action::Ui(UiMsg::SelectNext)).await;
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.state.ui.view == View::NowPlaying {
                    let _ = action_tx
                        .send(Action::Ui(UiMsg::ScrollNowPlaying(-3)))
                        .await;
                } else {
                    for _ in 0..3 {
                        let _ = action_tx.send(Action::Ui(UiMsg::SelectPrevious)).await;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // The header row is reserved for tab hit zones.
                if mouse.row == 0 {
                    let icons = crate::ui::icons::icons_for(self.state.ui.icon_mode);
                    let narrow =
                        crate::ui::layout::Breakpoint::from_width(self.state.ui.screen_area.width)
                            == crate::ui::layout::Breakpoint::Narrow;
                    for (view, start, end) in
                        crate::ui::header::tab_hit_zones(&icons, self.state.ui.view, narrow)
                    {
                        if mouse.column >= start && mouse.column < end {
                            let _ = action_tx.send(Action::Ui(UiMsg::Navigate(view))).await;
                            return;
                        }
                    }
                    return;
                }
                // A click on the now-playing progress row seeks by fraction.
                if self.state.has_now_playing() {
                    let layout =
                        crate::ui::layout::AppLayout::new(self.state.ui.screen_area, true, true);
                    let bar = layout.now_playing;
                    // The four-row chrome strip: rule, title, full-width
                    // timeline, status. The timeline is the third row.
                    let gauge_row = bar.y + 2;
                    if bar.height >= 4
                        && mouse.row == gauge_row
                        && mouse.column >= bar.x
                        && mouse.column < bar.x + bar.width
                    {
                        let fraction =
                            f64::from(mouse.column - bar.x) / f64::from(bar.width.max(1));
                        let _ = action_tx
                            .send(Action::Playback(PlaybackAction::SeekToFraction(fraction)))
                            .await;
                        return;
                    }
                }
                if self.state.ui.view == View::Home {
                    self.click_home(mouse.column, mouse.row, action_tx).await;
                    return;
                }
                self.click_list(mouse.column, mouse.row, action_tx).await;
            }
            _ => {}
        }
    }

    /// A click inside a Home item zone focuses its section and selects the
    /// item; the sections have no shared list geometry.
    async fn click_home(&mut self, column: u16, row: u16, action_tx: &mpsc::Sender<Action>) {
        let Some(zone) = self
            .state
            .ui
            .home_hit_zones
            .iter()
            .copied()
            .find(|zone| contains(zone.area, column, row))
        else {
            return;
        };
        if self.state.ui.home_section != zone.section {
            self.state.ui.home_section = zone.section;
            self.state.ui.selected_index = 0;
            self.state.reset_list();
        }
        // The Resume card is actionable without a list selection.
        if zone.section != HomeSection::Resume {
            if zone.index >= self.state.active_list_len() {
                return;
            }
            self.state.ui.selected_index = zone.index;
        }
        self.leave_text_focus();
        self.register_click(zone.index, action_tx).await;
    }

    /// A click on a row of the active list selects it; in the Playing view
    /// the queue pane also takes pane focus.
    async fn click_list(&mut self, column: u16, row: u16, action_tx: &mpsc::Sender<Action>) {
        let area = self.state.ui.list_hit_area;
        if !contains(area, column, row) {
            return;
        }
        // Every table view scrolls through `table_state`; only the playlist
        // master list uses `list_state`. Mapping a table view through the
        // stale list offset selected the wrong row on scrolled lists.
        let offset = self.state.ui.list_hit_offset.unwrap_or_else(|| {
            if self.state.ui.view == View::Playlists {
                self.state.ui.list_state.offset()
            } else {
                self.state.ui.table_state.offset()
            }
        });
        let index = offset + usize::from(row - area.y);
        // The Playing hit area is the queue pane; focus it before the
        // length check, which counts the queue only for the focused pane.
        if self.state.ui.view == View::NowPlaying {
            self.state.ui.playing_pane = PlayingPane::Queue;
        }
        if index >= self.state.active_list_len() {
            return;
        }
        self.state.ui.selected_index = index;
        self.leave_text_focus();
        self.register_click(index, action_tx).await;
    }

    /// Clicking a row moves keyboard focus back to content; an unlocked
    /// empty filter is dropped, exactly as Enter does in the filter bar.
    fn leave_text_focus(&mut self) {
        if self.state.ui.focus == Focus::Content {
            return;
        }
        if self.state.ui.focus == Focus::ListFilter
            && self
                .state
                .ui
                .list_filter
                .as_deref()
                .is_none_or(|filter| filter.trim().is_empty())
        {
            self.state.ui.list_filter = None;
        }
        self.state.ui.focus = Focus::Content;
    }

    /// Record one click on `index`; the second click on the same target
    /// within the window performs the view's Enter action verbatim.
    async fn register_click(&mut self, index: usize, action_tx: &mpsc::Sender<Action>) {
        let now = Instant::now();
        let target: ClickTarget = (self.state.ui.view, pane_fingerprint(&self.state), index);
        let double_click = self.last_click.is_some_and(|(at, view, pane, clicked)| {
            (view, pane, clicked) == target && now.duration_since(at) < DOUBLE_CLICK_WINDOW
        });
        if double_click {
            self.last_click = None;
            // Enter parity by construction: route the synthesized key
            // through the exact keyboard path (view bindings, channel
            // load-more, playing-pane rules).
            let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            self.handle_key(enter, action_tx).await;
        } else {
            self.last_click = Some((now, target.0, target.1, target.2));
        }
    }
}

/// Whether `area` contains the cell at (`column`, `row`).
fn contains(area: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= area.x && column < area.x + area.width && row >= area.y && row < area.y + area.height
}
