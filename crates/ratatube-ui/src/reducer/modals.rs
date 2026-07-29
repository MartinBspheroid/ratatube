//! Pure modal transitions: the universal track-context menu, details, and
//! the playlist picker/prompt/editor family.

use crate::state::{DomainState, UiState};
use ratatube_domain::action::{NavigationAction, PlaylistAction};
use ratatube_domain::effect::Effect;

/// Reduce context-menu intents that do not require application services.
pub fn reduce_track_context(
    ui: &mut UiState,
    domain: &DomainState,
    action: NavigationAction,
) -> Vec<Effect> {
    match action {
        NavigationAction::CloseTrackContext => ui.track_context_menu = None,
        NavigationAction::MoveTrackContext(delta) => {
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
        NavigationAction::ShowTrackDetails(track) => {
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
        NavigationAction::CloseTrackDetails => ui.track_details_modal = None,
        // Opening needs HistoryLog; submission dispatches the selected
        // stable action through existing action domains.
        NavigationAction::OpenTrackContext | NavigationAction::SubmitTrackContext => {}
        _ => unreachable!("non-context action routed to context reducer"),
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
        // PromptSubmit / ConfirmYes are resolved by the app layer, which
        // knows the services required by each purpose.
        _ => {}
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
