//! Playlist import lifecycle transitions.

use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::AppState;

/// Reduce playlist import lifecycle transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match Action::Playlists(action) {
        // --- Import --------------------------------------------------------
        Action::Playlists(PlaylistAction::StartImport(url)) => {
            state.ui.prompt = None;
            return vec![Effect::RunImport { url }];
        }
        Action::Playlists(PlaylistAction::ImportStarted { operation_id, url }) => {
            state.domain.import =
                Some(crate::app::state::ImportState::Fetching { operation_id, url });
        }
        Action::Playlists(PlaylistAction::ImportCompleted {
            operation_id,
            url,
            title,
            remote_id,
            tracks,
            rejections,
        }) => {
            if !matches!(
                state.domain.import,
                Some(crate::app::state::ImportState::Fetching {
                    operation_id: active,
                    ..
                }) if active == operation_id
            ) {
                return Vec::new();
            }
            let (playlist, summary) =
                crate::playlists::import::build_import(title, url, remote_id, tracks, rejections);
            state.domain.import = Some(crate::app::state::ImportState::Review {
                summary,
                playlist: Box::new(playlist),
            });
        }
        Action::Playlists(PlaylistAction::ImportFailed {
            operation_id,
            url,
            message,
        }) => {
            if !matches!(
                state.domain.import,
                Some(crate::app::state::ImportState::Fetching {
                    operation_id: active,
                    ..
                }) if active == operation_id
            ) {
                return Vec::new();
            }
            state.domain.import = Some(crate::app::state::ImportState::Failed {
                url,
                message: message.clone(),
            });
            state.notify(&format!("Import failed: {message}"), true);
        }
        Action::Playlists(PlaylistAction::CancelImport) => state.domain.import = None,
        // ConfirmImport is executed by the app layer (persists the playlist).
        _ => {}
    }
    Vec::new()
}
