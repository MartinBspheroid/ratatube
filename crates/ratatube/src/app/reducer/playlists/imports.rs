//! Playlist import lifecycle transitions.

use crate::app::action::PlaylistAction;
use crate::app::operations::OperationId;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, ImportState};

/// Reduce playlist import lifecycle transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaylistAction) -> Vec<Effect> {
    match action {
        PlaylistAction::StartImport(url) => {
            state.ui.prompt = None;
            return vec![Effect::RunImport { url }];
        }
        PlaylistAction::ImportStarted { operation_id, url } => {
            state.domain.import = Some(ImportState::Fetching { operation_id, url });
        }
        PlaylistAction::ImportCompleted {
            operation_id,
            url,
            title,
            remote_id,
            tracks,
            rejections,
        } => {
            import_completed(
                &mut state.domain,
                operation_id,
                ImportPayload {
                    url,
                    title,
                    remote_id,
                    tracks,
                    rejections,
                },
            );
        }
        PlaylistAction::ImportFailed {
            operation_id,
            url,
            message,
        } => {
            if import_failed(&mut state.domain, operation_id, url, &message) {
                state.notify(&format!("Import failed: {message}"), true);
            }
        }
        PlaylistAction::CancelImport => state.domain.import = None,
        // ConfirmImport is executed by the app layer (persists the playlist).
        _ => {}
    }
    Vec::new()
}

/// Raw import result fields, grouped to keep signatures within bounds.
struct ImportPayload {
    url: String,
    title: String,
    remote_id: Option<String>,
    tracks: Vec<crate::media::Track>,
    rejections: crate::media::yt_dlp::ImportRejections,
}

/// Move an in-flight import into review, only for the active operation.
fn import_completed(domain: &mut DomainState, operation_id: OperationId, payload: ImportPayload) {
    if !matches!(
        domain.import,
        Some(ImportState::Fetching {
            operation_id: active,
            ..
        }) if active == operation_id
    ) {
        return;
    }
    let (playlist, summary) = crate::playlists::import::build_import(
        payload.title,
        payload.url,
        payload.remote_id,
        payload.tracks,
        payload.rejections,
    );
    domain.import = Some(ImportState::Review {
        summary,
        playlist: Box::new(playlist),
    });
}

/// Record an import failure, only for the active operation.
fn import_failed(
    domain: &mut DomainState,
    operation_id: OperationId,
    url: String,
    message: &str,
) -> bool {
    if !matches!(
        domain.import,
        Some(ImportState::Fetching {
            operation_id: active,
            ..
        }) if active == operation_id
    ) {
        return false;
    }
    domain.import = Some(ImportState::Failed {
        url,
        message: message.to_string(),
    });
    true
}
