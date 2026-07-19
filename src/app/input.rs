//! Keyboard routing for text inputs, filters, and content views.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction};
use crate::app::state::{Focus, View};
use crate::input::keymap;

impl App {
    /// Route keyboard input: modals first, then focus-based keymap.
    pub(super) async fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        if self.handle_modal_key(&key, action_tx).await {
            return;
        }
        match self.state.focus {
            Focus::SearchInput => {
                if keymap::submits_search(&key) {
                    let query = self.state.search_input.trim().to_string();
                    self.submit_text_query(query, action_tx).await;
                    return;
                }
                if keymap::leaves_search(&key) {
                    self.state.focus = Focus::Content;
                    return;
                }
                if let Some(action) = keymap::search_input_action(&key) {
                    let _ = action_tx.send(action).await;
                }
            }
            Focus::ListFilter => match key.code {
                // Enter locks the filter so list keys operate on matches;
                // Esc clears the filter entirely.
                KeyCode::Enter => {
                    if self
                        .state
                        .list_filter
                        .as_deref()
                        .is_none_or(|filter| filter.trim().is_empty())
                    {
                        self.state.list_filter = None;
                    }
                    self.state.focus = Focus::Content;
                }
                KeyCode::Esc => {
                    self.state.list_filter = None;
                    self.state.focus = Focus::Content;
                }
                KeyCode::Backspace => {
                    if let Some(filter) = &mut self.state.list_filter
                        && filter.pop().is_none()
                    {
                        self.state.list_filter = None;
                        self.state.focus = Focus::Content;
                    }
                    self.state.selected_index = 0;
                }
                KeyCode::Char(character) => {
                    if let Some(filter) = &mut self.state.list_filter {
                        filter.push(character);
                    }
                    self.state.selected_index = 0;
                }
                // Allow movement while editing so users can filter, then pick,
                // without leaving the filter bar.
                KeyCode::Down => {
                    let _ = action_tx
                        .send(Action::Navigation(NavigationAction::SelectNext))
                        .await;
                }
                KeyCode::Up => {
                    let _ = action_tx
                        .send(Action::Navigation(NavigationAction::SelectPrevious))
                        .await;
                }
                _ => {}
            },
            Focus::Content => {
                if keymap::focuses_search(&key) {
                    // In list views `/` filters in place; elsewhere it moves
                    // focus to the Search tab.
                    if matches!(
                        self.state.view,
                        View::Queue | View::History | View::Playlists | View::PlaylistDetail
                    ) {
                        self.state.list_filter = Some(String::new());
                        self.state.focus = Focus::ListFilter;
                        self.state.selected_index = 0;
                    } else {
                        self.state.view = View::Search;
                        self.state.focus = Focus::SearchInput;
                    }
                    return;
                }
                // Esc clears a locked list filter and its derived index mapping.
                if key.code == KeyCode::Esc && self.state.list_filter.is_some() {
                    self.state.list_filter = None;
                    self.state.visible_indices = None;
                    self.state.clamp_selection();
                    return;
                }
                if self.state.view == View::NowPlaying
                    && let Some(action) = keymap::playing_pane_action(
                        &key,
                        self.state.playing_pane,
                        crate::ui::layout::Breakpoint::from_width(self.state.screen_area.width),
                    )
                {
                    let _ = action_tx.send(action).await;
                    return;
                }
                if let Some(action) = keymap::route(&key, Focus::Content, self.state.view) {
                    let _ = action_tx.send(action).await;
                }
            }
        }
    }
}
