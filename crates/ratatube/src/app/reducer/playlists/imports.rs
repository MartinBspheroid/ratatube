//! Playlist import lifecycle transitions.
//!
//! Every entry point takes the payload it needs rather than a
//! `PlaylistAction`, so the family dispatcher in `super` is the only place
//! that enumerates the enum.

use crate::app::operations::OperationId;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, ImportState};

/// Raw import result fields, grouped to keep signatures within bounds.
pub(super) struct ImportPayload {
    pub(super) url: String,
    pub(super) title: String,
    pub(super) remote_id: Option<String>,
    pub(super) tracks: Vec<crate::media::Track>,
    pub(super) rejections: crate::media::yt_dlp::ImportRejections,
}

/// Close the prompt and hand the URL to the supervised import effect.
pub(super) fn start_import(state: &mut AppState, url: String) -> Vec<Effect> {
    state.ui.prompt = None;
    vec![Effect::RunImport { url }]
}

/// Record the in-flight import, superseding any earlier one.
pub(super) fn import_started(
    state: &mut AppState,
    operation_id: OperationId,
    url: String,
) -> Vec<Effect> {
    state.domain.import = Some(ImportState::Fetching { operation_id, url });
    Vec::new()
}

/// Move an in-flight import into review, only for the active operation.
pub(super) fn import_completed(
    domain: &mut DomainState,
    operation_id: OperationId,
    payload: ImportPayload,
) -> Vec<Effect> {
    if !matches!(
        domain.import,
        Some(ImportState::Fetching {
            operation_id: active,
            ..
        }) if active == operation_id
    ) {
        return Vec::new();
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
    Vec::new()
}

/// Record an import failure and warn, only for the active operation.
pub(super) fn import_failed(
    state: &mut AppState,
    operation_id: OperationId,
    url: String,
    message: &str,
) -> Vec<Effect> {
    if record_failure(&mut state.domain, operation_id, url, message) {
        state.notify(&format!("Import failed: {message}"), true);
    }
    Vec::new()
}

/// Drop the import card without persisting anything.
pub(super) fn cancel_import(state: &mut AppState) -> Vec<Effect> {
    state.domain.import = None;
    Vec::new()
}

/// Record an import failure, only for the active operation.
fn record_failure(
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
