//! Root-state list selection and derived presentation helpers.

use crate::app::state::{AppState, HomeSection, PlayingPane, View};
use crate::media::search::SearchState;
use crate::ui::layout::Breakpoint;

impl AppState {
    /// Keep playlist presentation and selection order newest-updated first.
    pub fn sort_playlists_by_updated(&mut self) {
        self.playlists
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }

    /// Whether the now-playing bar should render.
    pub fn has_now_playing(&self) -> bool {
        self.current_track.is_some()
    }

    /// Chapters of the current track.
    pub fn chapters(&self) -> &[crate::media::Chapter] {
        self.current_details
            .as_ref()
            .map_or(&[], |details| details.chapters.as_slice())
    }

    /// Index of the chapter containing the current playhead.
    pub fn current_chapter_index(&self) -> Option<usize> {
        crate::media::chapter_at(self.chapters(), self.playback.position_seconds)
    }

    /// Clamp selection to the length of the active list.
    pub fn clamp_selection(&mut self) {
        let length = self.active_list_len();
        if length == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= length {
            self.selected_index = length - 1;
        }
    }

    /// Reset list scroll state after navigation.
    pub fn reset_list(&mut self) {
        self.list_state = ratatui::widgets::ListState::default();
        self.table_state = ratatui::widgets::TableState::default();
        self.clamp_selection();
    }

    /// Advance the spinner animation frame.
    pub fn tick_spinner(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    /// Map a filtered position back to its underlying-list index.
    pub fn resolve_index(&self, visible: usize) -> usize {
        match &self.visible_indices {
            Some(indices) => indices.get(visible).copied().unwrap_or(visible),
            None => visible,
        }
    }

    /// Length of the list backing the current view.
    pub fn active_list_len(&self) -> usize {
        if let Some(indices) = &self.visible_indices {
            return indices.len();
        }
        match self.view {
            View::Home => match self.home_section {
                HomeSection::Resume => 0,
                HomeSection::Recent => self.home_recent_len,
                HomeSection::Playlists => self.playlists.len(),
            },
            View::Search => match &self.search {
                SearchState::Results { tracks, .. } => tracks.len(),
                _ => 0,
            },
            View::Queue => self.queue.order.len(),
            View::Playlists => self.playlists.len(),
            View::PlaylistDetail => self
                .selected_playlist
                .and_then(|index| self.playlists.get(index))
                .map_or(0, |playlist| playlist.tracks.len()),
            View::History => self.history_len,
            View::NowPlaying
                if self.playing_pane == PlayingPane::Queue
                    && Breakpoint::from_width(self.screen_area.width) == Breakpoint::UltraWide =>
            {
                self.queue.order.len()
            }
            View::NowPlaying | View::Help => 0,
        }
    }
}
