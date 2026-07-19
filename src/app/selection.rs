//! Filter synchronization and selected-track resolution across views.

use crate::app::state::{HistoryViewMode, View};
use crate::app::{App, FilterSyncKey};
use crate::media::Track;

impl App {
    /// Recompute the filtered view of the active list and mirror the History
    /// length for its presentation mode. The runtime calls this before render.
    pub(super) fn sync_list_view(&mut self) {
        self.state.history_len = match self.state.history_view_mode {
            HistoryViewMode::Recent => self
                .history
                .as_ref()
                .map_or(0, |history| history.recent_unique_indices().len()),
            HistoryViewMode::Top => self
                .history
                .as_ref()
                .map_or(0, |history| history.aggregate().len()),
        };

        let filter = self
            .state
            .list_filter
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let sync_key = FilterSyncKey {
            view: self.state.view,
            history_mode: self.state.history_view_mode,
            filter: filter.clone(),
            list_revision: self.list_revision,
        };
        if self.filter_sync_key.as_ref() == Some(&sync_key) {
            return;
        }
        self.filter_sync_key = Some(sync_key);
        if filter.is_empty() {
            self.state.visible_indices = match (
                self.state.view,
                self.state.history_view_mode,
                self.history.as_ref(),
            ) {
                (View::History, HistoryViewMode::Recent, Some(history)) => {
                    Some(history.recent_unique_indices())
                }
                _ => None,
            };
            self.state.clamp_selection();
            return;
        }
        if self.state.view == View::History
            && self.state.history_view_mode == HistoryViewMode::Recent
        {
            self.state.visible_indices = self.history.as_ref().map(|history| {
                history
                    .recent_unique_indices()
                    .into_iter()
                    .filter(|&index| {
                        let entry = &history.entries()[index];
                        crate::app::filter::matches(
                            &filter,
                            &format!("{} {}", entry.artist, entry.title),
                            Some(&format!("{:?}", entry.outcome)),
                        )
                    })
                    .collect()
            });
            self.state.clamp_selection();
            return;
        }
        let rows: Vec<(String, Option<String>)> = match self.state.view {
            View::Queue => self
                .state
                .queue
                .order
                .iter()
                .map(|&index| {
                    let track = &self.state.queue.tracks[index];
                    (format!("{} {}", track.artist, track.title), None)
                })
                .collect(),
            View::Playlists => self
                .state
                .playlists
                .iter()
                .map(|playlist| (playlist.name.clone(), None))
                .collect(),
            View::PlaylistDetail => self
                .state
                .selected_playlist
                .and_then(|index| self.state.playlists.get(index))
                .map(|playlist| {
                    playlist
                        .tracks
                        .iter()
                        .map(|track| (format!("{} {}", track.artist, track.title), None))
                        .collect()
                })
                .unwrap_or_default(),
            View::History => match (&self.history, self.state.history_view_mode) {
                (Some(history), HistoryViewMode::Recent) => history
                    .entries()
                    .iter()
                    .map(|entry| {
                        (
                            format!("{} {}", entry.artist, entry.title),
                            Some(format!("{:?}", entry.outcome)),
                        )
                    })
                    .collect(),
                (Some(history), HistoryViewMode::Top) => history
                    .aggregate()
                    .iter()
                    .map(|summary| {
                        (
                            format!("{} {}", summary.entry.artist, summary.entry.title),
                            Some(format!("{:?}", summary.entry.outcome)),
                        )
                    })
                    .collect(),
                (None, _) => Vec::new(),
            },
            _ => {
                self.state.visible_indices = None;
                return;
            }
        };
        self.state.visible_indices = Some(crate::app::filter::matching_indices(
            &filter,
            rows.iter()
                .map(|(text, outcome)| (text.clone(), outcome.as_deref())),
        ));
        self.state.clamp_selection();
    }

    /// Resolve the selected track across track-listing views, mapping through
    /// the in-list filter and History presentation mode.
    pub(super) fn resolve_selected_track(&self) -> Option<Track> {
        let index = self.state.resolve_index(self.state.selected_index);
        match self.state.view {
            View::Home => match self.state.home_section {
                crate::app::state::HomeSection::Recent => {
                    self.history.as_ref().and_then(|history| {
                        history
                            .recent_unique(self.state.selected_index + 1)
                            .into_iter()
                            .nth(self.state.selected_index)
                    })
                }
                crate::app::state::HomeSection::Resume
                | crate::app::state::HomeSection::Playlists => None,
            },
            View::History => match self.state.history_view_mode {
                HistoryViewMode::Recent => self
                    .history
                    .as_ref()
                    .and_then(|history| history.entries().get(index).map(|entry| entry.to_track())),
                HistoryViewMode::Top => self.history.as_ref().and_then(|history| {
                    history
                        .aggregate()
                        .get(index)
                        .map(|summary| summary.entry.to_track())
                }),
            },
            View::Search => match &self.state.search {
                crate::media::search::SearchState::Results { tracks, .. } => {
                    tracks.get(index).cloned()
                }
                _ => None,
            },
            View::Queue => self
                .state
                .queue
                .order
                .get(index)
                .map(|&queue_index| self.state.queue.tracks[queue_index].clone()),
            View::PlaylistDetail => self
                .state
                .selected_playlist
                .and_then(|playlist_index| self.state.playlists.get(playlist_index))
                .and_then(|playlist| playlist.tracks.get(index))
                .map(Track::from),
            View::Channel => self
                .state
                .channel
                .as_ref()
                .and_then(|channel| channel.tracks.get(index))
                .cloned(),
            _ => None,
        }
    }
}
