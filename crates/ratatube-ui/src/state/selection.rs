//! List selection and derived presentation helpers, hosted on the state half
//! that owns them: domain facts on [`DomainState`], selection and scroll on
//! [`UiState`], with thin [`AppState`] wrappers preserving call sites.

use crate::render::layout::Breakpoint;
use crate::state::{AppState, DomainState, HomeSection, PlayingPane, UiState, View};
use ratatube_domain::media::search::SearchState;

impl UiState {
    /// Clamp selection to the length of the active list.
    pub fn clamp_selection(&mut self, domain: &DomainState) {
        let length = self.active_list_len(domain);
        if length == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= length {
            self.selected_index = length - 1;
        }
    }

    /// Reset list scroll state; called on navigation between views.
    pub fn reset_list(&mut self, domain: &DomainState) {
        self.list_state = ratatui::widgets::ListState::default();
        self.table_state = ratatui::widgets::TableState::default();
        self.clamp_selection(domain);
    }

    /// Advance the spinner animation frame.
    pub fn tick_spinner(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    /// Map a position in the possibly filtered visible list back to an index
    /// into the underlying list.
    pub fn resolve_index(&self, visible: usize) -> usize {
        match &self.visible_indices {
            Some(indices) => indices.get(visible).copied().unwrap_or(visible),
            None => visible,
        }
    }

    /// Length of the list backing the current view.
    pub fn active_list_len(&self, domain: &DomainState) -> usize {
        if let Some(indices) = &self.visible_indices {
            return indices.len();
        }
        match self.view {
            View::Home => match self.home_section {
                HomeSection::Resume => 0,
                HomeSection::Recent => self.home_recent_len,
                HomeSection::Playlists => domain.playlists.len(),
            },
            View::Search => match &domain.search {
                SearchState::Results { tracks, .. } => tracks.len(),
                _ => 0,
            },
            View::Queue => domain.queue.order.len(),
            View::Playlists => domain.playlists.len(),
            View::PlaylistDetail => self
                .selected_playlist
                .and_then(|index| domain.playlists.get(index))
                .map_or(0, |playlist| playlist.tracks.len()),
            View::Channel => domain.channel.as_ref().map_or(0, |channel| {
                channel.tracks.len() + usize::from(!channel.exhausted && !channel.loading)
            }),
            View::History => self.history_len,
            View::NowPlaying
                if self.playing_pane == PlayingPane::Queue
                    && Breakpoint::from_width(self.screen_area.width) == Breakpoint::UltraWide =>
            {
                domain.queue.order.len()
            }
            View::NowPlaying | View::Help => 0,
        }
    }
}

/// Thin wrappers so numerous existing call sites keep compiling; the real
/// bodies live on the owning half above.
impl AppState {
    pub fn sync_track_transition(&mut self, now: std::time::Instant) {
        self.domain.sync_track_transition(now);
    }

    /// Keep playlist presentation and selection order newest-updated first.
    pub fn sort_playlists_by_updated(&mut self) {
        self.domain.sort_playlists_by_updated();
    }

    /// Whether the now-playing bar should render (PRD section 8).
    pub fn has_now_playing(&self) -> bool {
        self.domain.has_now_playing()
    }

    /// Chapters of the current track, whether uploader-set or parsed from a tracklist.
    pub fn chapters(&self) -> &[ratatube_domain::media::Chapter] {
        self.domain.chapters()
    }

    /// Index of the chapter the playhead is currently inside.
    pub fn current_chapter_index(&self) -> Option<usize> {
        self.domain.current_chapter_index()
    }

    /// Clamp selection to the length of the active list.
    pub fn clamp_selection(&mut self) {
        self.ui.clamp_selection(&self.domain);
    }

    /// Reset list scroll state; called on navigation between views.
    pub fn reset_list(&mut self) {
        self.ui.reset_list(&self.domain);
    }

    /// Advance the spinner animation frame.
    pub fn tick_spinner(&mut self) {
        self.ui.tick_spinner();
    }

    /// Map a position in the possibly filtered visible list back to an index
    /// into the underlying list.
    pub fn resolve_index(&self, visible: usize) -> usize {
        self.ui.resolve_index(visible)
    }

    /// Length of the list backing the current view.
    pub fn active_list_len(&self) -> usize {
        self.ui.active_list_len(&self.domain)
    }
}
