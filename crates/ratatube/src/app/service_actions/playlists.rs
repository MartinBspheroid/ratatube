//! Playlist service-action routing by editing versus storage workflow.
//!
//! The match is wildcard-free on purpose: every `PlaylistAction` variant —
//! including the by-id variants the daemon receives over the wire — must be
//! given an owner here or the crate does not compile. A catch-all in this
//! position once dropped by-id playlist commands silently (348e200).

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
            // Storage owns loading, deletion, prompts, and import persistence.
            PlaylistAction::DeleteSelectedPlaylist => {
                self.confirm_delete_selected_playlist(action_tx).await;
            }
            PlaylistAction::LoadPlaylistIntoQueue(id) => {
                self.load_playlist_into_queue(&id, action_tx).await;
            }
            PlaylistAction::AppendPlaylistToQueue(id) => {
                self.append_playlist_to_queue(&id, action_tx).await;
            }
            PlaylistAction::ImportPlaylistsJson(json) => self.import_playlists_json(&json),
            PlaylistAction::CreatePlaylist(name) => self.create_playlist(&name),
            PlaylistAction::SaveQueueAsPlaylist(name) => self.save_queue_as_playlist(&name),
            PlaylistAction::RenameSelectedPlaylist(name) => self.rename_selected_playlist(&name),
            PlaylistAction::DeletePlaylistConfirmed(id) => self.delete_playlist(&id),
            PlaylistAction::ConfirmYes => self.dispatch_confirmed_action(action_tx).await,
            PlaylistAction::PromptSubmit => self.submit_prompt(action_tx).await,
            PlaylistAction::ConfirmImport => self.confirm_import(action_tx).await,

            // Editing owns the picker, metadata editor, removals, and reordering.
            PlaylistAction::OpenPlaylistPicker => self.open_playlist_picker(),
            PlaylistAction::OpenPlaylistPickerForTrack(track) => {
                self.state.show_playlist_picker(track);
            }
            PlaylistAction::PickerSubmit => self.submit_picker().await,
            PlaylistAction::RemoveSelectedFromPlaylist => self.remove_selected_from_playlist(),
            PlaylistAction::RemoveTrackOccurrence {
                playlist_id,
                track_index,
                expected_track,
                expected_revision,
            } => self.remove_playlist_occurrence(
                &playlist_id,
                track_index,
                &expected_track,
                expected_revision,
            ),
            PlaylistAction::PlaylistEditorSubmit => self.submit_playlist_editor(),
            PlaylistAction::AddTrackToPlaylist { playlist_id, track } => {
                self.add_track_to_playlist(&playlist_id, track);
            }
            PlaylistAction::AddTrackToNewPlaylist { name, track } => {
                self.add_track_to_new_playlist(&name, track);
            }
            PlaylistAction::RenamePlaylist { id, name } => {
                self.edit_playlist_by_id(&id, &name, None);
            }
            PlaylistAction::EditPlaylist {
                id,
                name,
                description,
            } => self.edit_playlist_by_id(&id, &name, Some(&description)),
            PlaylistAction::MoveTrackInPlaylist { id, from, to } => {
                self.move_track_in_playlist(&id, from, to);
            }
            PlaylistAction::MoveSelectedInPlaylist(delta) => self.move_selected_in_playlist(delta),

            // Catalog navigation, modal text entry, and supervised imports are
            // fully handled by the reducer; they need no service work.
            PlaylistAction::OpenPlaylistDetail
            | PlaylistAction::DeletePlaylist(_)
            | PlaylistAction::PlaylistSaved(_)
            | PlaylistAction::StartImport(_)
            | PlaylistAction::ImportStarted { .. }
            | PlaylistAction::ImportCompleted { .. }
            | PlaylistAction::ImportFailed { .. }
            | PlaylistAction::CancelImport
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
            | PlaylistAction::ConfirmNo
            | PlaylistAction::PickerInput(_)
            | PlaylistAction::PickerBackspace
            | PlaylistAction::PickerNext
            | PlaylistAction::PickerPrevious
            | PlaylistAction::PickerCancel => {}
        }
    }
}
