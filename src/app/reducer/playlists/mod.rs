//! Playlist reducer routing by sub-responsibility.

mod catalog;
mod imports;

use crate::app::action::PlaylistAction;
use crate::app::reducer::Effect;
use crate::app::state::AppState;

/// Route a playlist action to its focused reducer.
pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match action {
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
        action @ (PlaylistAction::OpenPlaylistDetail
        | PlaylistAction::PlaylistSaved(_)
        | PlaylistAction::DeletePlaylist(_)) => catalog::reduce(state, action),
        action @ (PlaylistAction::StartImport(_)
        | PlaylistAction::ImportStarted { .. }
        | PlaylistAction::ImportCompleted { .. }
        | PlaylistAction::ImportFailed { .. }
        | PlaylistAction::CancelImport) => imports::reduce(state, action),
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
