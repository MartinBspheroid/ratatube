//! Root-state list selection and derived presentation helpers.

use crate::app::state::{AppState, HomeSection, PlayingPane, View};
use crate::media::search::SearchState;
use crate::ui::layout::Breakpoint;

impl AppState {
    /// Reconcile final-window timing with the current playback and queue facts.
    pub(crate) fn sync_track_transition(&mut self, now: std::time::Instant) {
        let occurrence = (self.current_track.is_some() && self.playback_occurrence != 0)
            .then_some(self.playback_occurrence);
        let timing_is_current = occurrence.is_some()
            && self.position_occurrence == occurrence
            && self.duration_occurrence == occurrence;
        let remaining_seconds = timing_is_current
            .then_some(self.playback.duration_seconds)
            .flatten()
            .filter(|duration| duration.is_finite() && *duration >= 0.0)
            .zip(
                self.playback
                    .position_seconds
                    .is_finite()
                    .then_some(self.playback.position_seconds),
            )
            .map(|(duration, position)| (duration - position).max(0.0));
        self.track_transition.update(
            crate::playback::TransitionInput {
                occurrence,
                remaining_seconds,
                playing: self.playback.status == crate::playback::PlaybackStatus::Playing,
                has_next: self.queue.effective_next().is_some(),
            },
            now,
        );
    }

    /// Start a distinct playback occurrence and invalidate all prior timing.
    pub(crate) fn begin_playback_occurrence(&mut self) {
        self.playback_occurrence = self.playback_occurrence.wrapping_add(1).max(1);
        self.playback_loaded_occurrence = None;
        self.position_occurrence = None;
        self.duration_occurrence = None;
        self.playback.position_seconds = 0.0;
        self.playback.duration_seconds = None;
    }

    /// Mark mpv's genuine media-load boundary for the accepted occurrence.
    pub(crate) fn mark_file_loaded(&mut self) {
        if self.playback_occurrence != 0 {
            self.playback_loaded_occurrence = Some(self.playback_occurrence);
        }
    }

    /// Record position only after the active load's `file-loaded` boundary.
    pub(crate) fn record_position(&mut self, position: f64) {
        if self.playback_loaded_occurrence == Some(self.playback_occurrence) {
            self.playback.position_seconds = position;
            self.position_occurrence = Some(self.playback_occurrence);
        }
    }

    /// Record duration only after the active load's `file-loaded` boundary.
    pub(crate) fn record_duration(&mut self, duration: f64) {
        if self.playback_loaded_occurrence == Some(self.playback_occurrence) {
            self.playback.duration_seconds = Some(duration);
            self.duration_occurrence = Some(self.playback_occurrence);
        }
    }

    /// Keep playlist presentation and selection order newest-updated first.
    pub fn sort_playlists_by_updated(&mut self) {
        self.playlists
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }

    /// Whether the now-playing bar should render (PRD section 8).
    pub fn has_now_playing(&self) -> bool {
        self.current_track.is_some()
    }

    /// Chapters of the current track, whether uploader-set or parsed from a tracklist.
    pub fn chapters(&self) -> &[crate::media::Chapter] {
        self.current_details
            .as_ref()
            .map_or(&[], |details| details.chapters.as_slice())
    }

    /// Index of the chapter the playhead is currently inside.
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

    /// Reset list scroll state; called on navigation between views.
    pub fn reset_list(&mut self) {
        self.list_state = ratatui::widgets::ListState::default();
        self.table_state = ratatui::widgets::TableState::default();
        self.clamp_selection();
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
            View::Channel => self.channel.as_ref().map_or(0, |channel| {
                channel.tracks.len() + usize::from(!channel.exhausted && !channel.loading)
            }),
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
