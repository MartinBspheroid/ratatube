//! Playlist catalog transitions.

use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, View};
use crate::playlists::Playlist;

/// Reduce playlist catalog state transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match action {
        PlaylistAction::OpenPlaylistDetail => {
            if state.ui.view == View::Playlists && !state.domain.playlists.is_empty() {
                state.ui.selected_playlist = Some(state.resolve_index(state.ui.selected_index));
                state.ui.view = View::PlaylistDetail;
                state.ui.selected_index = 0;
                state.ui.list_filter = None;
                state.ui.visible_indices = None;
            }
        }
        PlaylistAction::PlaylistSaved(playlist) => playlist_saved(&mut state.domain, playlist),
        PlaylistAction::DeletePlaylist(id) => {
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

/// Upsert a saved playlist and re-sort the catalog newest-updated first.
fn playlist_saved(domain: &mut DomainState, playlist: Playlist) {
    match domain.playlists.iter().position(|p| p.id == playlist.id) {
        Some(i) => domain.playlists[i] = playlist,
        None => domain.playlists.push(playlist),
    }
    domain.bump_playlists_revision();
    domain.sort_playlists_by_updated();
}
