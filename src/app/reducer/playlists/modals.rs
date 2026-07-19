//! Playlist picker, prompt, editor, and confirmation transitions.

use crate::app::action::{Action, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::AppState;

pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match Action::Playlists(action) {
        Action::Playlists(PlaylistAction::PickerInput(c)) => {
            if let Some(picker) = &mut state.picker {
                picker.filter.push(c);
                picker.selected = 0;
            }
        }
        Action::Playlists(PlaylistAction::PickerBackspace) => {
            if let Some(picker) = &mut state.picker {
                picker.filter.pop();
                picker.selected = 0;
            }
        }
        Action::Playlists(PlaylistAction::PickerNext) => picker_move(state, 1),
        Action::Playlists(PlaylistAction::PickerPrevious) => picker_move(state, -1),
        Action::Playlists(PlaylistAction::PickerCancel) => state.picker = None,
        Action::Playlists(PlaylistAction::OpenPrompt(purpose)) => {
            state.prompt = Some(crate::app::state::PromptState {
                purpose,
                buffer: String::new(),
            });
        }
        Action::Playlists(PlaylistAction::PromptInput(c)) => {
            if let Some(prompt) = &mut state.prompt
                && prompt.buffer.len() < 1_048_576
            {
                prompt.buffer.push(c);
            }
        }
        Action::Playlists(PlaylistAction::PromptPaste(text)) => {
            if let Some(prompt) = &mut state.prompt {
                let remaining = 1_048_576usize.saturating_sub(prompt.buffer.len());
                prompt.buffer.extend(text.chars().take(remaining));
            }
        }
        Action::Playlists(PlaylistAction::PromptBackspace) => {
            if let Some(prompt) = &mut state.prompt {
                prompt.buffer.pop();
            }
        }
        Action::Playlists(PlaylistAction::PromptCancel) => state.prompt = None,
        Action::Playlists(PlaylistAction::OpenPlaylistEditor) => {
            state.playlist_editor = state
                .selected_playlist
                .and_then(|index| state.playlists.get(index))
                .map(|playlist| crate::app::state::PlaylistEditorState {
                    name: playlist.name.clone(),
                    description: playlist.description.clone(),
                    field: crate::app::state::PlaylistEditorField::Name,
                });
        }
        Action::Playlists(PlaylistAction::PlaylistEditorInput(character)) => {
            if let Some(editor) = &mut state.playlist_editor {
                match editor.field {
                    crate::app::state::PlaylistEditorField::Name => editor.name.push(character),
                    crate::app::state::PlaylistEditorField::Description => {
                        editor.description.push(character);
                    }
                }
            }
        }
        Action::Playlists(PlaylistAction::PlaylistEditorBackspace) => {
            if let Some(editor) = &mut state.playlist_editor {
                match editor.field {
                    crate::app::state::PlaylistEditorField::Name => {
                        editor.name.pop();
                    }
                    crate::app::state::PlaylistEditorField::Description => {
                        editor.description.pop();
                    }
                }
            }
        }
        Action::Playlists(PlaylistAction::PlaylistEditorNextField) => {
            if let Some(editor) = &mut state.playlist_editor {
                editor.field = match editor.field {
                    crate::app::state::PlaylistEditorField::Name => {
                        crate::app::state::PlaylistEditorField::Description
                    }
                    crate::app::state::PlaylistEditorField::Description => {
                        crate::app::state::PlaylistEditorField::Name
                    }
                };
            }
        }
        Action::Playlists(PlaylistAction::PlaylistEditorCancel) => state.playlist_editor = None,
        Action::Playlists(PlaylistAction::ConfirmNo) => state.confirm = None,
        // PromptSubmit / ConfirmYes are resolved by the app layer, which
        // knows the services required by each purpose.

        // --- Playlists ----------------------------------------------------
        _ => {}
    }
    Vec::new()
}

/// Move the picker selection through its candidate list, wrapping.
fn picker_move(state: &mut AppState, delta: i32) {
    let Some(picker) = &state.picker else {
        return;
    };
    let (create_new, matching) =
        crate::app::filter::picker_candidates(&state.playlists, &picker.filter);
    let total = usize::from(create_new) + matching.len();
    if total > 0
        && let Some(picker) = &mut state.picker
    {
        picker.selected = (picker.selected as i32 + delta).rem_euclid(total as i32) as usize;
    }
}
