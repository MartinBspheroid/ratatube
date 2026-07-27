//! Playlist service-action routing by editing versus storage workflow.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaylistAction};

impl App {
    /// Route playlist service actions to editing or storage ownership.
    pub(super) async fn handle_playlist_service(
        &mut self,
        action: PlaylistAction,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match action {
            // Editing owns selection-based flows plus the by-id variants the
            // daemon receives over the wire; a by-id action missing here is
            // silently dropped by storage's catch-all.
            action @ (PlaylistAction::OpenPlaylistPicker
            | PlaylistAction::OpenPlaylistPickerForTrack(_)
            | PlaylistAction::PickerSubmit
            | PlaylistAction::RemoveSelectedFromPlaylist
            | PlaylistAction::RemoveTrackOccurrence { .. }
            | PlaylistAction::PlaylistEditorSubmit
            | PlaylistAction::MoveSelectedInPlaylist(_)
            | PlaylistAction::AddTrackToPlaylist { .. }
            | PlaylistAction::AddTrackToNewPlaylist { .. }
            | PlaylistAction::RenamePlaylist { .. }
            | PlaylistAction::EditPlaylist { .. }
            | PlaylistAction::MoveTrackInPlaylist { .. }) => {
                self.handle_playlist_editing(action).await;
            }
            action => self.handle_playlist_storage(action, action_tx).await,
        }
    }
}
