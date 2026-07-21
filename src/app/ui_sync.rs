//! The single point where UI state reacts to domain changes.
//!
//! Phase 2 replaces the in-process event `Vec` with socket broadcasts; this
//! function then runs on the client against its `DomainMirror`.

use crate::app::domain_event::DomainEvent;
use crate::app::state::{DomainState, UiState};

/// Apply universal UI invariants for a batch of domain events. Reactions
/// must be idempotent: coordinators may already have applied the specific
/// ones (selection clamps after removals, notifications).
pub(crate) fn apply_domain_events(domain: &DomainState, ui: &mut UiState, events: &[DomainEvent]) {
    let clamps = events.iter().any(|event| {
        matches!(
            event,
            DomainEvent::QueueChanged
                | DomainEvent::PlaylistsChanged
                | DomainEvent::SearchChanged
                | DomainEvent::HistoryChanged
                | DomainEvent::ChannelChanged
        )
    });
    if clamps {
        ui.clamp_selection(domain);
    }
}
