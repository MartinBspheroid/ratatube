//! Playlist catalog transitions.
//!
//! Every entry point takes the payload it needs rather than a
//! `PlaylistAction`, so the family dispatcher in `super` is the only place
//! that enumerates the enum.

use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, View};
use crate::playlists::Playlist;

/// Open the detail view for the selected playlist.
pub(super) fn open_playlist_detail(state: &mut AppState) -> Vec<Effect> {
    if state.ui.view == View::Playlists && !state.domain.playlists.is_empty() {
        state.ui.selected_playlist = Some(state.resolve_index(state.ui.selected_index));
        state.ui.view = View::PlaylistDetail;
        state.ui.selected_index = 0;
        state.ui.list_filter = None;
        state.ui.visible_indices = None;
    }
    Vec::new()
}

/// Upsert a saved playlist and re-sort the catalog newest-updated first.
pub(super) fn playlist_saved(domain: &mut DomainState, playlist: Playlist) -> Vec<Effect> {
    match domain.playlists.iter().position(|p| p.id == playlist.id) {
        Some(i) => domain.playlists[i] = playlist,
        None => domain.playlists.push(playlist),
    }
    domain.bump_playlists_revision();
    domain.sort_playlists_by_updated();
    Vec::new()
}

/// Arm the delete confirmation; the deletion itself waits for a yes.
pub(super) fn confirm_delete_playlist(state: &mut AppState, id: String) -> Vec<Effect> {
    state.ui.confirm = Some(crate::app::state::ConfirmState {
        message: "Delete this playlist? (local file only, y/n)".to_string(),
        action: Box::new(Action::Playlists(PlaylistAction::DeletePlaylistConfirmed(
            id,
        ))),
    });
    Vec::new()
}
