//! Playlist catalog transitions.

use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, View};

/// Reduce playlist catalog state transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match Action::Playlists(action) {
        Action::Playlists(PlaylistAction::OpenPlaylistDetail) => {
            if state.ui.view == View::Playlists && !state.domain.playlists.is_empty() {
                state.ui.selected_playlist = Some(state.resolve_index(state.ui.selected_index));
                state.ui.view = View::PlaylistDetail;
                state.ui.selected_index = 0;
                state.ui.list_filter = None;
                state.ui.visible_indices = None;
            }
        }
        Action::Playlists(PlaylistAction::PlaylistSaved(playlist)) => {
            match state
                .domain
                .playlists
                .iter()
                .position(|p| p.id == playlist.id)
            {
                Some(i) => state.domain.playlists[i] = playlist,
                None => state.domain.playlists.push(playlist),
            }
            state.bump_playlists_revision();
            state.sort_playlists_by_updated();
        }
        Action::Playlists(PlaylistAction::DeletePlaylist(id)) => {
            state.ui.confirm = Some(crate::app::state::ConfirmState {
                message: "Delete this playlist? (local file only, y/n)".to_string(),
                action: Box::new(Action::Playlists(PlaylistAction::DeletePlaylistConfirmed(
                    id,
                ))),
            });
        }

        _ => {}
    }
    Vec::new()
}
