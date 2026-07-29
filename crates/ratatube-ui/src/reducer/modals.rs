//! Pure modal transitions: the universal track-context menu, details, and
//! the playlist picker/prompt/editor family.

use crate::state::{DomainState, UiState};
use ratatube_domain::action::{NavigationAction, PlaylistAction};
use ratatube_domain::effect::Effect;
use ratatube_domain::media::Track;

/// The context-menu subset of [`NavigationAction`]: the only messages
/// [`reduce_context_menu`] can be asked to apply. Narrowing the input this
/// way makes "non-context action routed to the context reducer" an
/// unrepresentable state instead of a runtime panic.
#[derive(Debug, Clone)]
pub enum TrackContextMsg {
    /// Resolve the selected track and open the menu (service-owned).
    Open,
    /// Close the menu without executing an action.
    Close,
    /// Move the menu selection by a signed number of rows.
    Move(i32),
    /// Submit the selected menu row (service-owned).
    Submit,
    /// Show details for a track without changing playback ownership.
    ShowDetails(Track),
    /// Close the selected-track details modal.
    CloseDetails,
}

impl TrackContextMsg {
    /// Classify one navigation action, yielding `None` for the variants the
    /// navigation coordinator routes to search, channel, or quit handling.
    ///
    /// Wildcard-free on purpose: a new [`NavigationAction`] variant cannot
    /// compile until it is classified as context-menu or not.
    pub fn from_navigation(action: NavigationAction) -> Option<Self> {
        match action {
            NavigationAction::OpenTrackContext => Some(Self::Open),
            NavigationAction::CloseTrackContext => Some(Self::Close),
            NavigationAction::MoveTrackContext(delta) => Some(Self::Move(delta)),
            NavigationAction::SubmitTrackContext => Some(Self::Submit),
            NavigationAction::ShowTrackDetails(track) => Some(Self::ShowDetails(track)),
            NavigationAction::CloseTrackDetails => Some(Self::CloseDetails),
            NavigationAction::Quit
            | NavigationAction::SearchInput(_)
            | NavigationAction::SearchBackspace
            | NavigationAction::SubmitSearch(_)
            | NavigationAction::SubmitExactVideo(_)
            | NavigationAction::SearchCompleted { .. }
            | NavigationAction::SearchFailed { .. }
            | NavigationAction::ClearSearch
            | NavigationAction::OpenInBrowser
            | NavigationAction::ExternalCommandCompleted { .. }
            | NavigationAction::VisitChannel(_)
            | NavigationAction::ChannelResolved { .. }
            | NavigationAction::ChannelPageLoaded { .. }
            | NavigationAction::LoadMoreChannel
            | NavigationAction::RetryChannel
            | NavigationAction::BackFromChannel => None,
        }
    }
}

/// Reduce context-menu intents that do not require application services.
///
/// Stays total over [`NavigationAction`] so the navigation coordinator can
/// forward its context-menu arm as-is, but the transitions themselves take
/// the narrowed [`TrackContextMsg`]. A non-context action is a routing
/// decision owned by that coordinator's own wildcard-free match, so it is
/// left unhandled here: this is public API of the crate and must not abort
/// the process for a caller that misroutes.
pub fn reduce_track_context(
    ui: &mut UiState,
    domain: &DomainState,
    action: NavigationAction,
) -> Vec<Effect> {
    match TrackContextMsg::from_navigation(action) {
        Some(msg) => reduce_context_menu(ui, domain, msg),
        None => Vec::new(),
    }
}

/// Apply one context-menu or details transition.
pub fn reduce_context_menu(
    ui: &mut UiState,
    domain: &DomainState,
    msg: TrackContextMsg,
) -> Vec<Effect> {
    match msg {
        TrackContextMsg::Close => ui.track_context_menu = None,
        TrackContextMsg::Move(delta) => {
            if let Some(menu) = &mut ui.track_context_menu {
                let len = menu.context.actions.len();
                if len > 0 {
                    let selected = menu.selected % len;
                    let delta = (i64::from(delta)).rem_euclid(len as i64) as usize;
                    menu.selected = if selected >= len - delta {
                        selected - (len - delta)
                    } else {
                        selected + delta
                    };
                }
            }
        }
        TrackContextMsg::ShowDetails(track) => {
            // Existing extended details apply only when they belong to this
            // exact track (mirrors the pre-split AppState helper).
            let details = domain
                .current_track
                .as_ref()
                .is_some_and(|current| current.id == track.id)
                .then(|| domain.current_details.clone())
                .flatten();
            ui.show_track_details(track, details);
        }
        TrackContextMsg::CloseDetails => ui.track_details_modal = None,
        // Opening needs HistoryLog; submission dispatches the selected
        // stable action through existing action domains.
        TrackContextMsg::Open | TrackContextMsg::Submit => {}
    }
    Vec::new()
}

/// Reduce picker, prompt, editor, and confirmation modal transitions.
pub fn reduce_playlist_modals(
    ui: &mut UiState,
    domain: &DomainState,
    action: PlaylistAction,
) -> Vec<Effect> {
    match action {
        PlaylistAction::PickerInput(c) => {
            if let Some(picker) = &mut ui.picker {
                picker.filter.push(c);
                picker.selected = 0;
            }
        }
        PlaylistAction::PickerBackspace => {
            if let Some(picker) = &mut ui.picker {
                picker.filter.pop();
                picker.selected = 0;
            }
        }
        PlaylistAction::PickerNext => picker_move(ui, domain, 1),
        PlaylistAction::PickerPrevious => picker_move(ui, domain, -1),
        PlaylistAction::PickerCancel => ui.picker = None,
        PlaylistAction::OpenPrompt(purpose) => {
            ui.prompt = Some(crate::state::PromptState {
                purpose,
                buffer: String::new(),
            });
        }
        PlaylistAction::PromptInput(c) => {
            if let Some(prompt) = &mut ui.prompt
                && prompt.buffer.len() < 1_048_576
            {
                prompt.buffer.push(c);
            }
        }
        PlaylistAction::PromptPaste(text) => {
            if let Some(prompt) = &mut ui.prompt {
                let remaining = 1_048_576usize.saturating_sub(prompt.buffer.len());
                prompt.buffer.extend(text.chars().take(remaining));
            }
        }
        PlaylistAction::PromptBackspace => {
            if let Some(prompt) = &mut ui.prompt {
                prompt.buffer.pop();
            }
        }
        PlaylistAction::PromptCancel => ui.prompt = None,
        PlaylistAction::OpenPlaylistEditor => {
            ui.playlist_editor = ui
                .selected_playlist
                .and_then(|index| domain.playlists.get(index))
                .map(|playlist| crate::state::PlaylistEditorState {
                    name: playlist.name.clone(),
                    description: playlist.description.clone(),
                    field: crate::state::PlaylistEditorField::Name,
                });
        }
        PlaylistAction::PlaylistEditorInput(character) => {
            if let Some(editor) = &mut ui.playlist_editor {
                match editor.field {
                    crate::state::PlaylistEditorField::Name => editor.name.push(character),
                    crate::state::PlaylistEditorField::Description => {
                        editor.description.push(character);
                    }
                }
            }
        }
        PlaylistAction::PlaylistEditorBackspace => {
            if let Some(editor) = &mut ui.playlist_editor {
                match editor.field {
                    crate::state::PlaylistEditorField::Name => {
                        editor.name.pop();
                    }
                    crate::state::PlaylistEditorField::Description => {
                        editor.description.pop();
                    }
                }
            }
        }
        PlaylistAction::PlaylistEditorNextField => {
            if let Some(editor) = &mut ui.playlist_editor {
                editor.field = match editor.field {
                    crate::state::PlaylistEditorField::Name => {
                        crate::state::PlaylistEditorField::Description
                    }
                    crate::state::PlaylistEditorField::Description => {
                        crate::state::PlaylistEditorField::Name
                    }
                };
            }
        }
        PlaylistAction::PlaylistEditorCancel => ui.playlist_editor = None,
        PlaylistAction::ConfirmNo => ui.confirm = None,
        // Everything else is owned by the playlist coordinator or the service
        // layer: submissions (`PromptSubmit`, `PickerSubmit`, `ConfirmYes`,
        // `PlaylistEditorSubmit`) need the services each purpose implies, and
        // the catalog, import, and by-id commands never touch modal state.
        // Listed explicitly so a new variant cannot be silently dropped.
        PlaylistAction::DeleteSelectedPlaylist
        | PlaylistAction::OpenPlaylistDetail
        | PlaylistAction::SaveQueueAsPlaylist(_)
        | PlaylistAction::CreatePlaylist(_)
        | PlaylistAction::RenameSelectedPlaylist(_)
        | PlaylistAction::LoadPlaylistIntoQueue(_)
        | PlaylistAction::AppendPlaylistToQueue(_)
        | PlaylistAction::DeletePlaylist(_)
        | PlaylistAction::DeletePlaylistConfirmed(_)
        | PlaylistAction::PlaylistSaved(_)
        | PlaylistAction::StartImport(_)
        | PlaylistAction::ImportStarted { .. }
        | PlaylistAction::ImportCompleted { .. }
        | PlaylistAction::ImportFailed { .. }
        | PlaylistAction::ConfirmImport
        | PlaylistAction::CancelImport
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
        | PlaylistAction::MoveSelectedInPlaylist(_) => {}
    }
    Vec::new()
}

/// Move the picker selection through its candidate list, wrapping.
fn picker_move(ui: &mut UiState, domain: &DomainState, delta: i32) {
    let Some(picker) = &ui.picker else {
        return;
    };
    let (create_new, matching) =
        crate::filter::picker_candidates(&domain.playlists, &picker.filter);
    let total = usize::from(create_new) + matching.len();
    if total > 0
        && let Some(picker) = &mut ui.picker
    {
        picker.selected = (picker.selected as i32 + delta).rem_euclid(total as i32) as usize;
    }
}
