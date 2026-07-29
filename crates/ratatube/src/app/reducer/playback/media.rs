//! Playback metadata and chapter transitions; pane toggles route to the UI
//! reducer.
//!
//! Every entry point takes the payload it needs rather than a
//! `PlaybackAction`, so the family dispatcher in `super` is the only place
//! that enumerates the enum.

use crate::app::operations::OperationId;
use crate::app::reducer::Effect;
use crate::app::state::{DetailsStatus, DomainState};

/// Mark an extended-metadata fetch as the active one.
pub(super) fn details_started(
    domain: &mut DomainState,
    operation_id: OperationId,
    track_id: String,
) -> Vec<Effect> {
    domain.details_status = DetailsStatus::Loading {
        operation_id,
        track_id,
    };
    Vec::new()
}

/// Apply details only when they still belong to the active operation and the
/// current track.
pub(super) fn details_loaded(
    domain: &mut DomainState,
    operation_id: OperationId,
    track_id: String,
    details: crate::media::TrackDetails,
) -> Vec<Effect> {
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
    Vec::new()
}

/// Record a details failure only for the active operation and current track.
pub(super) fn details_failed(
    domain: &mut DomainState,
    operation_id: OperationId,
    track_id: String,
    message: String,
) -> Vec<Effect> {
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
    Vec::new()
}

/// Seek to the next chapter boundary after the playhead.
pub(super) fn next_chapter(domain: &DomainState) -> Vec<Effect> {
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
pub(super) fn previous_chapter(domain: &DomainState) -> Vec<Effect> {
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
