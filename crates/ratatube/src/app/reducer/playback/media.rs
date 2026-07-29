//! Playback metadata and chapter transitions; pane toggles route to the UI
//! reducer.

use crate::app::action::PlaybackAction;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DetailsStatus, DomainState};

/// Reduce metadata, chapter, and playback-pane transitions.
pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match action {
        PlaybackAction::DetailsStarted {
            operation_id,
            track_id,
        } => {
            state.domain.details_status = DetailsStatus::Loading {
                operation_id,
                track_id,
            };
            Vec::new()
        }
        PlaybackAction::DetailsLoaded {
            operation_id,
            track_id,
            details,
        } => {
            details_loaded(&mut state.domain, operation_id, track_id, *details);
            Vec::new()
        }
        PlaybackAction::DetailsFailed {
            operation_id,
            track_id,
            message,
        } => {
            details_failed(&mut state.domain, operation_id, track_id, message);
            Vec::new()
        }
        PlaybackAction::NextChapter => next_chapter(&state.domain),
        PlaybackAction::PreviousChapter => previous_chapter(&state.domain),
        _ => Vec::new(),
    }
}

/// Apply details only when they still belong to the active operation and the
/// current track.
fn details_loaded(
    domain: &mut DomainState,
    operation_id: crate::app::operations::OperationId,
    track_id: String,
    details: crate::media::TrackDetails,
) {
    if matches!(
        domain.details_status,
        DetailsStatus::Loading {
            operation_id: active,
            track_id: ref active_track_id,
        } if active == operation_id && active_track_id == &track_id
    ) && domain.current_track.as_ref().map(|t| t.id.as_str()) == Some(track_id.as_str())
    {
        domain.current_details = Some(details);
        domain.details_status = DetailsStatus::Idle;
    }
}

/// Record a details failure only for the active operation and current track.
fn details_failed(
    domain: &mut DomainState,
    operation_id: crate::app::operations::OperationId,
    track_id: String,
    message: String,
) {
    if matches!(
        domain.details_status,
        DetailsStatus::Loading {
            operation_id: active,
            track_id: ref active_track_id,
        } if active == operation_id && active_track_id == &track_id
    ) && domain.current_track.as_ref().map(|t| t.id.as_str()) == Some(track_id.as_str())
    {
        domain.details_status = DetailsStatus::Failed { track_id, message };
    }
}

/// Seek to the next chapter boundary after the playhead.
fn next_chapter(domain: &DomainState) -> Vec<Effect> {
    let position = domain.playback.position_seconds;
    if let Some(chapter) = domain
        .chapters()
        .iter()
        .find(|c| c.start_seconds > position + 1.0)
    {
        return vec![Effect::SeekTo(chapter.start_seconds)];
    }
    Vec::new()
}

/// Restart the current chapter, or step back to the previous one.
fn previous_chapter(domain: &DomainState) -> Vec<Effect> {
    let chapters = domain.chapters();
    if let Some(current) = domain.current_chapter_index() {
        let start = chapters[current].start_seconds;
        // Like PreviousTrack: restart the current chapter first, then step
        // back to the one before it.
        let target = if domain.playback.position_seconds > start + 3.0 || current == 0 {
            start
        } else {
            chapters[current - 1].start_seconds
        };
        return vec![Effect::SeekTo(target)];
    }
    Vec::new()
}
