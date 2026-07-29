//! Broadcast-shaped domain change notifications.
//!
//! Phase 1 applies these in-process through `apply_domain_events`; Phase 2
//! sends the same events over the daemon socket to every connected client.

use crate::app::action::{Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction};
use crate::app::state::DomainState;

pub use ratatube_domain::event::DomainEvent;

/// Cheap pre-reduce facts used to derive change events afterwards.
pub(crate) struct DomainWatermark {
    queue_revision: u64,
    queue_position: Option<usize>,
    playlists_revision: u64,
    current_track_id: Option<String>,
    playback_occurrence: u64,
    mpv_ready: bool,
    yt_dlp_ready: bool,
}

impl DomainWatermark {
    /// Capture the counters and identities that mark domain changes.
    pub(crate) fn capture(domain: &DomainState) -> Self {
        Self {
            queue_revision: domain.queue_revision,
            queue_position: domain.queue.position,
            playlists_revision: domain.playlists_revision,
            current_track_id: domain.current_track.as_ref().map(|t| t.id.clone()),
            playback_occurrence: domain.playback_occurrence,
            mpv_ready: domain.mpv_ready,
            yt_dlp_ready: domain.yt_dlp_ready,
        }
    }

    /// Derive the events implied by the difference to the current state,
    /// plus action families whose changes have no counter.
    pub(crate) fn events_since(&self, domain: &DomainState, action: &Action) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        if domain.queue_revision != self.queue_revision
            || domain.queue.position != self.queue_position
        {
            events.push(DomainEvent::QueueChanged);
        }
        if domain.playlists_revision != self.playlists_revision {
            events.push(DomainEvent::PlaylistsChanged);
        }
        if domain.current_track.as_ref().map(|t| t.id.as_str()) != self.current_track_id.as_deref()
            || domain.playback_occurrence != self.playback_occurrence
        {
            events.push(DomainEvent::TrackChanged);
        }
        if domain.mpv_ready != self.mpv_ready || domain.yt_dlp_ready != self.yt_dlp_ready {
            events.push(DomainEvent::Health);
        }
        if let Some(event) = counterless_event(action) {
            events.push(event);
        }
        if records_history(action) {
            events.push(DomainEvent::HistoryChanged);
        }
        events
    }
}

/// Actions whose service hooks record playback history (skips, stops, and
/// natural track endings); the store lives outside `DomainState`, so no
/// counter can observe it.
fn records_history(action: &Action) -> bool {
    matches!(
        action,
        Action::Playback(
            PlaybackAction::NextTrack
                | PlaybackAction::PreviousTrack
                | PlaybackAction::Stop
                | PlaybackAction::PlayTrack(_)
                | PlaybackAction::PlaySelected
        )
    ) || matches!(
        action,
        Action::Playback(PlaybackAction::PlaybackEvent(
            crate::playback::PlaybackEvent::EndFile { .. }
        ))
    )
}

/// Action families that mutate domain state without a revision counter.
fn counterless_event(action: &Action) -> Option<DomainEvent> {
    match action {
        Action::Navigation(
            NavigationAction::SubmitSearch(_)
            | NavigationAction::SubmitExactVideo(_)
            | NavigationAction::ClearSearch
            | NavigationAction::SearchCompleted { .. }
            | NavigationAction::SearchFailed { .. },
        ) => Some(DomainEvent::SearchChanged),
        Action::Navigation(
            NavigationAction::VisitChannel(_)
            | NavigationAction::ChannelResolved { .. }
            | NavigationAction::ChannelPageLoaded { .. }
            | NavigationAction::LoadMoreChannel
            | NavigationAction::RetryChannel
            | NavigationAction::BackFromChannel,
        ) => Some(DomainEvent::ChannelChanged),
        Action::Playback(PlaybackAction::PlaybackEvent(_)) => Some(DomainEvent::PlaybackChanged),
        Action::Playback(
            PlaybackAction::DetailsLoaded { .. } | PlaybackAction::DetailsFailed { .. },
        ) => Some(DomainEvent::TrackDetailsChanged),
        Action::Playlists(
            PlaylistAction::ImportStarted { .. }
            | PlaylistAction::ImportCompleted { .. }
            | PlaylistAction::ImportFailed { .. }
            | PlaylistAction::CancelImport,
        ) => Some(DomainEvent::ImportChanged),
        Action::History(
            HistoryAction::ClearActivity
            | HistoryAction::ClearHistoryConfirmed
            | HistoryAction::DeleteSelectedHistoryEntry,
        ) => Some(DomainEvent::HistoryChanged),
        _ => None,
    }
}
