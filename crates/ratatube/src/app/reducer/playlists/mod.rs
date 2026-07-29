//! Playlist reducer routing by sub-responsibility.
//!
//! This match is the single enumeration of [`PlaylistAction`] on the reducer
//! side and it is wildcard-free on purpose: a new variant cannot compile until
//! it is given an owner here, so it can never silently reduce to nothing (the
//! failure mode 348e200 shipped). The sub-reducers take the payload they need
//! instead of the whole enum, which is what makes a catch-all unnecessary
//! rather than merely hidden.

mod catalog;
mod imports;

use crate::app::action::PlaylistAction;
use crate::app::reducer::Effect;
use crate::app::state::AppState;

/// Route a playlist action to its focused reducer.
pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match action {
        // Modal text entry and picker/prompt/editor navigation are pure
        // presentation; the UI reducer owns them and matches on the action.
        action @ (PlaylistAction::PickerInput(_)
        | PlaylistAction::PickerBackspace
        | PlaylistAction::PickerNext
        | PlaylistAction::PickerPrevious
        | PlaylistAction::PickerCancel
        | PlaylistAction::OpenPrompt(_)
        | PlaylistAction::PromptInput(_)
        | PlaylistAction::PromptPaste(_)
        | PlaylistAction::PromptBackspace
        | PlaylistAction::PromptCancel
        | PlaylistAction::OpenPlaylistEditor
        | PlaylistAction::PlaylistEditorInput(_)
        | PlaylistAction::PlaylistEditorBackspace
        | PlaylistAction::PlaylistEditorNextField
        | PlaylistAction::PlaylistEditorCancel
        | PlaylistAction::ConfirmNo) => crate::app::reducer::ui::modals::reduce_playlist_modals(
            &mut state.ui,
            &state.domain,
            action,
        ),

        // Catalog navigation and upserts.
        PlaylistAction::OpenPlaylistDetail => catalog::open_playlist_detail(state),
        PlaylistAction::PlaylistSaved(playlist) => {
            catalog::playlist_saved(&mut state.domain, playlist)
        }
        PlaylistAction::DeletePlaylist(id) => catalog::confirm_delete_playlist(state, id),

        // Import lifecycle.
        PlaylistAction::StartImport(url) => imports::start_import(state, url),
        PlaylistAction::ImportStarted { operation_id, url } => {
            imports::import_started(state, operation_id, url)
        }
        PlaylistAction::ImportCompleted {
            operation_id,
            url,
            title,
            remote_id,
            tracks,
            rejections,
        } => imports::import_completed(
            &mut state.domain,
            operation_id,
            imports::ImportPayload {
                url,
                title,
                remote_id,
                tracks,
                rejections,
            },
        ),
        PlaylistAction::ImportFailed {
            operation_id,
            url,
            message,
        } => imports::import_failed(state, operation_id, url, &message),
        PlaylistAction::CancelImport => imports::cancel_import(state),

        // Store-backed workflows: creating, renaming, deleting, loading and
        // appending playlists, the picker/editor/prompt submits, and
        // `ConfirmImport` all need the playlist store, so `service_actions`
        // applies them after reduce. There is no pure transition to make here.
        PlaylistAction::DeleteSelectedPlaylist
        | PlaylistAction::SaveQueueAsPlaylist(_)
        | PlaylistAction::CreatePlaylist(_)
        | PlaylistAction::RenameSelectedPlaylist(_)
        | PlaylistAction::LoadPlaylistIntoQueue(_)
        | PlaylistAction::AppendPlaylistToQueue(_)
        | PlaylistAction::DeletePlaylistConfirmed(_)
        | PlaylistAction::ConfirmImport
        | PlaylistAction::PromptSubmit
        | PlaylistAction::PlaylistEditorSubmit
        | PlaylistAction::ConfirmYes
        | PlaylistAction::OpenPlaylistPicker
        | PlaylistAction::OpenPlaylistPickerForTrack(_)
        | PlaylistAction::PickerSubmit
        | PlaylistAction::RemoveSelectedFromPlaylist
        | PlaylistAction::RemoveTrackOccurrence { .. }
        | PlaylistAction::AddTrackToPlaylist { .. }
        | PlaylistAction::AddTrackToNewPlaylist { .. }
        | PlaylistAction::RenamePlaylist { .. }
        | PlaylistAction::EditPlaylist { .. }
        | PlaylistAction::MoveTrackInPlaylist { .. }
        | PlaylistAction::ImportPlaylistsJson(_)
        | PlaylistAction::MoveSelectedInPlaylist(_) => Vec::new(),
    }
}
