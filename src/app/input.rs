//! Keyboard routing for text inputs, filters, and content views.

use crate::app::actions::UiMsg;
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
        if let Some(capture) = self.state.modal_capture() {
            self.handle_modal_key(capture, &key, action_tx).await;
            return;
        }
        match self.state.ui.focus {
            Focus::SearchInput => {
                if keymap::submits_search(&key) {
                    let query = self.state.ui.search_input.trim().to_string();
                    self.submit_text_query(query, action_tx).await;
                    return;
                }
                if keymap::leaves_search(&key) {
                    self.state.ui.focus = Focus::Content;
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
                        .ui
                        .list_filter
                        .as_deref()
                        .is_none_or(|filter| filter.trim().is_empty())
                    {
                        self.state.ui.list_filter = None;
                    }
                    self.state.ui.focus = Focus::Content;
                }
                KeyCode::Esc => {
                    self.state.ui.list_filter = None;
                    self.state.ui.focus = Focus::Content;
                }
                KeyCode::Backspace => {
                    if let Some(filter) = &mut self.state.ui.list_filter
                        && filter.pop().is_none()
                    {
                        self.state.ui.list_filter = None;
                        self.state.ui.focus = Focus::Content;
                    }
                    self.state.ui.selected_index = 0;
                }
                KeyCode::Char(character) => {
                    if let Some(filter) = &mut self.state.ui.list_filter {
                        filter.push(character);
                    }
                    self.state.ui.selected_index = 0;
                }
                // Allow movement while editing so users can filter, then pick,
                // without leaving the filter bar.
                KeyCode::Down => {
                    let _ = action_tx.send(Action::Ui(UiMsg::SelectNext)).await;
                }
                KeyCode::Up => {
                    let _ = action_tx.send(Action::Ui(UiMsg::SelectPrevious)).await;
                }
                _ => {}
            },
            Focus::Content => {
                if self.state.ui.view == View::Channel
                    && key.code == KeyCode::Enter
                    && let Some(channel) = self.state.domain.channel.as_ref()
                    && self.state.ui.selected_index == channel.tracks.len()
                    && !channel.exhausted
                    && !channel.loading
                {
                    let action = if channel.error.is_some() {
                        NavigationAction::RetryChannel
                    } else {
                        NavigationAction::LoadMoreChannel
                    };
                    let _ = action_tx.send(Action::Navigation(action)).await;
                    return;
                }
                if keymap::focuses_search(&key) {
                    // In list views `/` filters in place; elsewhere it moves
                    // focus to the Search tab.
                    if matches!(
                        self.state.ui.view,
                        View::Queue | View::History | View::Playlists | View::PlaylistDetail
                    ) {
                        self.state.ui.list_filter = Some(String::new());
                        self.state.ui.focus = Focus::ListFilter;
                        self.state.ui.selected_index = 0;
                    } else {
                        self.state.ui.view = View::Search;
                        self.state.ui.focus = Focus::SearchInput;
                    }
                    return;
                }
                // Esc clears a locked list filter and its derived index mapping.
                if key.code == KeyCode::Esc && self.state.ui.list_filter.is_some() {
                    self.state.ui.list_filter = None;
                    self.state.ui.visible_indices = None;
                    self.state.clamp_selection();
                    return;
                }
                if self.state.ui.view == View::NowPlaying
                    && let Some(action) = keymap::playing_pane_action(
                        &key,
                        self.state.ui.playing_pane,
                        crate::ui::layout::Breakpoint::from_width(self.state.ui.screen_area.width),
                    )
                {
                    let _ = action_tx.send(action).await;
                    return;
                }
                if let Some(action) = keymap::route(&key, Focus::Content, self.state.ui.view) {
                    if matches!(
                        action,
                        Action::Navigation(NavigationAction::OpenTrackContext)
                    ) {
                        self.handle_action(action, action_tx).await;
                    } else {
                        let _ = action_tx.send(action).await;
                    }
                }
            }
        }
    }
}
