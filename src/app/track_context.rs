//! Resolve the selected track and its source-specific context actions.

use crate::app::state::{AppState, HistoryViewMode, HomeSection, View};
use crate::history::HistoryService;
use crate::media::Track;
use crate::media::search::SearchState;

/// Surface and exact occurrence from which a track context was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackSource {
    Search,
    Queue {
        order_index: usize,
    },
    Playlist {
        playlist_id: String,
        track_index: usize,
    },
    History,
    /// A video selected from the dedicated Channel view.
    Channel,
    Playing,
    Home,
}

/// Stable data describing one operation offered by a track context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackContextAction {
    PlayNow,
    PlayNext,
    AddToQueue,
    AddToPlaylist,
    VisitChannel,
    ShowDetails,
    OpenInBrowser,
    CopyUrl,
    RemoveFromQueue {
        order_index: usize,
    },
    RemoveFromPlaylist {
        playlist_id: String,
        track_index: usize,
    },
}

/// Selected track, its exact source, and ordered valid operations.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackContext {
    pub track: Track,
    pub source: TrackSource,
    pub actions: Vec<TrackContextAction>,
    /// Collection generation captured with a removable occurrence.
    pub collection_revision: Option<CollectionRevision>,
}

/// Collection generation that distinguishes otherwise identical occurrences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionRevision {
    Queue(u64),
    Playlists(u64),
}

/// Resolve the selected track for every currently available track-bearing view.
pub fn resolve_track_context(
    state: &AppState,
    history: Option<&HistoryService>,
) -> Option<TrackContext> {
    let selected = selected_track_index(state)?;
    let (track, source) = match state.ui.view {
        View::Search => match &state.domain.search {
            SearchState::Results { tracks, .. } => {
                (tracks.get(selected)?.clone(), TrackSource::Search)
            }
            _ => return None,
        },
        View::Queue => {
            let track_index = *state.domain.queue.order.get(selected)?;
            let track = state.domain.queue.tracks.get(track_index)?.clone();
            (
                track,
                TrackSource::Queue {
                    order_index: selected,
                },
            )
        }
        View::PlaylistDetail => {
            let playlist = state
                .ui
                .selected_playlist
                .and_then(|index| state.domain.playlists.get(index))?;
            let track = Track::from(playlist.tracks.get(selected)?);
            (
                track,
                TrackSource::Playlist {
                    playlist_id: playlist.id.clone(),
                    track_index: selected,
                },
            )
        }
        View::Channel => {
            let track = state.domain.channel.as_ref()?.tracks.get(selected)?.clone();
            (track, TrackSource::Channel)
        }
        View::History => {
            let history = history?;
            let track = match state.ui.history_view_mode {
                HistoryViewMode::Recent => history.entries().get(selected)?.to_track(),
                HistoryViewMode::Top => history.aggregate().get(selected)?.entry.to_track(),
            };
            (track, TrackSource::History)
        }
        View::NowPlaying
            if state.ui.playing_pane == crate::app::state::PlayingPane::Queue
                && crate::ui::layout::Breakpoint::from_width(state.ui.screen_area.width)
                    == crate::ui::layout::Breakpoint::UltraWide =>
        {
            let track_index = *state.domain.queue.order.get(selected)?;
            let track = state.domain.queue.tracks.get(track_index)?.clone();
            (
                track,
                TrackSource::Queue {
                    order_index: selected,
                },
            )
        }
        View::NowPlaying => (state.domain.current_track.clone()?, TrackSource::Playing),
        View::Home if state.ui.home_section == HomeSection::Recent => {
            let history = history?;
            let entry_index = *history.recent_unique_indices().get(selected)?;
            (
                history.entries().get(entry_index)?.to_track(),
                TrackSource::Home,
            )
        }
        View::Home | View::Playlists | View::Help => return None,
    };
    let actions = resolve_actions(state, &track, &source);
    let collection_revision = match source {
        TrackSource::Queue { .. } => Some(CollectionRevision::Queue(state.domain.queue_revision)),
        TrackSource::Playlist { .. } => Some(CollectionRevision::Playlists(
            state.domain.playlists_revision,
        )),
        _ => None,
    };
    Some(TrackContext {
        track,
        source,
        actions,
        collection_revision,
    })
}

fn selected_track_index(state: &AppState) -> Option<usize> {
    match &state.ui.visible_indices {
        Some(indices) => indices.get(state.ui.selected_index).copied(),
        None => Some(state.ui.selected_index),
    }
}

fn resolve_actions(
    state: &AppState,
    track: &Track,
    source: &TrackSource,
) -> Vec<TrackContextAction> {
    let mut actions = vec![TrackContextAction::PlayNow, TrackContextAction::PlayNext];
    if !state
        .domain
        .queue
        .tracks
        .iter()
        .any(|queued| queued.id == track.id)
    {
        actions.push(TrackContextAction::AddToQueue);
    }
    // Legacy tracks still offer VisitChannel. The navigation service resolves
    // channel metadata from `track.webpage_url` before opening the channel;
    // stored channel identity is only an optimization, not an applicability gate.
    actions.extend([
        TrackContextAction::AddToPlaylist,
        TrackContextAction::VisitChannel,
        TrackContextAction::ShowDetails,
        TrackContextAction::OpenInBrowser,
        TrackContextAction::CopyUrl,
    ]);
    match source {
        TrackSource::Queue { order_index } => {
            actions.push(TrackContextAction::RemoveFromQueue {
                order_index: *order_index,
            });
        }
        TrackSource::Playlist {
            playlist_id,
            track_index,
        } => actions.push(TrackContextAction::RemoveFromPlaylist {
            playlist_id: playlist_id.clone(),
            track_index: *track_index,
        }),
        TrackSource::Search
        | TrackSource::History
        | TrackSource::Channel
        | TrackSource::Playing
        | TrackSource::Home => {}
    }
    actions
}

/// Open a resolved menu or report the exact empty-selection error.
pub(super) fn open_track_context(state: &mut AppState, history: Option<&HistoryService>) {
    match resolve_track_context(state, history) {
        Some(context) => {
            state.ui.track_context_generation =
                state.ui.track_context_generation.wrapping_add(1).max(1);
            state.ui.track_context_menu = Some(crate::app::state::TrackContextMenuState {
                context,
                selected: 0,
            });
        }
        None => {
            state.ui.track_context_menu = None;
            state.notify("No track selected", true);
        }
    }
}

#[cfg(test)]
mod tests;
