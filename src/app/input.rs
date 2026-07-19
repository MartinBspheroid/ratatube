//! Keyboard routing for overlays, text inputs, filters, and content views.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction, PlaylistAction};
use crate::app::state::{Focus, ImportState, View};
use crate::input::keymap;

impl App {
    /// Route keyboard input: modals first, then focus-based keymap.
    pub(super) async fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        action_tx: &mpsc::Sender<Action>,
    ) {
        // The notification log is modal; any documented dismiss key closes it.
        if self.state.show_notification_log {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('!') | KeyCode::Char('q') | KeyCode::Enter
            ) {
                self.state.show_notification_log = false;
            }
            return;
        }
        if self.state.search_detail_open && matches!(key.code, KeyCode::Esc | KeyCode::Char('i')) {
            let _ = action_tx
                .send(Action::Navigation(NavigationAction::ToggleSearchDetail))
                .await;
            return;
        }
        // Esc always cancels an import modal; Enter confirms only a successfully
        // fetched review, never a loading or failed import.
        if let Some(import) = &self.state.import {
            let action = match (import, key.code) {
                (ImportState::Review { .. }, KeyCode::Enter) => {
                    Some(Action::Playlists(PlaylistAction::ConfirmImport))
                }
                (_, KeyCode::Esc) => Some(Action::Playlists(PlaylistAction::CancelImport)),
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        // Confirmation dialogs accept only explicit yes/no input.
        if self.state.confirm.is_some() {
            let action = match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    Some(Action::Playlists(PlaylistAction::ConfirmYes))
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    Some(Action::Playlists(PlaylistAction::ConfirmNo))
                }
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        if self.state.playlist_editor.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::Playlists(PlaylistAction::PlaylistEditorSubmit)),
                KeyCode::Esc => Some(Action::Playlists(PlaylistAction::PlaylistEditorCancel)),
                KeyCode::Tab | KeyCode::BackTab => {
                    Some(Action::Playlists(PlaylistAction::PlaylistEditorNextField))
                }
                KeyCode::Backspace => {
                    Some(Action::Playlists(PlaylistAction::PlaylistEditorBackspace))
                }
                KeyCode::Char(character) => Some(Action::Playlists(
                    PlaylistAction::PlaylistEditorInput(character),
                )),
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        // Text prompt modal.
        if self.state.prompt.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::Playlists(PlaylistAction::PromptSubmit)),
                KeyCode::Esc => Some(Action::Playlists(PlaylistAction::PromptCancel)),
                KeyCode::Backspace => Some(Action::Playlists(PlaylistAction::PromptBackspace)),
                KeyCode::Char(character) => {
                    Some(Action::Playlists(PlaylistAction::PromptInput(character)))
                }
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
            return;
        }
        // Add-to-playlist picker modal.
        if self.state.picker.is_some() {
            let action = match key.code {
                KeyCode::Enter => Some(Action::Playlists(PlaylistAction::PickerSubmit)),
                KeyCode::Esc => Some(Action::Playlists(PlaylistAction::PickerCancel)),
                KeyCode::Backspace => Some(Action::Playlists(PlaylistAction::PickerBackspace)),
                KeyCode::Down => Some(Action::Playlists(PlaylistAction::PickerNext)),
                KeyCode::Up => Some(Action::Playlists(PlaylistAction::PickerPrevious)),
                KeyCode::Char(character) => {
                    Some(Action::Playlists(PlaylistAction::PickerInput(character)))
                }
                _ => None,
            };
            if let Some(action) = action {
                let _ = action_tx.send(action).await;
            }
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
