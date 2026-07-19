//! Playlist catalog transitions.

use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, View};

pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match Action::Playlists(action) {
        Action::Playlists(PlaylistAction::OpenPlaylistDetail) => {
            if state.view == View::Playlists && !state.playlists.is_empty() {
                state.selected_playlist = Some(state.resolve_index(state.selected_index));
                state.view = View::PlaylistDetail;
                state.selected_index = 0;
                state.list_filter = None;
                state.visible_indices = None;
            }
        }
        Action::Playlists(PlaylistAction::PlaylistSaved(playlist)) => {
            match state.playlists.iter().position(|p| p.id == playlist.id) {
                Some(i) => state.playlists[i] = playlist,
                None => state.playlists.push(playlist),
            }
            state.sort_playlists_by_updated();
        }
        Action::Playlists(PlaylistAction::DeletePlaylist(id)) => {
            state.confirm = Some(crate::app::state::ConfirmState {
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
