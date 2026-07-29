//! Queue-family coordinator: membership and order mutations live on
//! [`DomainState`]; selection, filter repair, and notifications stay UI-side
//! glue in the coordinator.

use crate::app::action::{Action, QueueAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState, View};
use crate::media::Track;

/// Result of a guarded occurrence removal.
enum OccurrenceRemoval {
    /// The queue changed since the menu captured the occurrence.
    Cancelled,
    /// The exact occurrence was removed.
    Removed,
    /// The occurrence was already gone.
    Missing,
}

/// Reduce queue-domain state transitions.
pub(super) fn reduce(state: &mut AppState, action: QueueAction) -> Vec<Effect> {
    match action {
        QueueAction::AddToQueue(track) => {
            let effects = add_to_queue(&mut state.domain, track);
            state.notify("Added to queue", false);
            effects
        }
        QueueAction::AddNext(track) => {
            let effects = add_next(&mut state.domain, track);
            state.notify("Will play next", false);
            effects
        }
        QueueAction::RemoveSelectedFromQueue => {
            if state.ui.view != View::Queue {
                return Vec::new();
            }
            let real = state.resolve_index(state.ui.selected_index);
            if remove_at(&mut state.domain, real) {
                state.notify("Removed from queue — u to undo", false);
            }
            // The filter indices refresh next loop; drop them now so the
            // stale mapping can't resolve a second removal wrongly.
            if let Some(indices) = &mut state.ui.visible_indices {
                indices.retain(|&i| i != real);
                for i in indices.iter_mut() {
                    if *i > real {
                        *i -= 1;
                    }
                }
            }
            state.clamp_selection();
            vec![Effect::PersistQueue]
        }
        QueueAction::RemoveTrackOccurrence {
            order_index,
            expected_track,
            expected_revision,
        } => match remove_occurrence(
            &mut state.domain,
            order_index,
            &expected_track,
            expected_revision,
        ) {
            OccurrenceRemoval::Cancelled => {
                state.notify("Queue changed; removal cancelled", true);
                Vec::new()
            }
            OccurrenceRemoval::Removed => {
                state.notify("Removed from queue — u to undo", false);
                state.ui.visible_indices = None;
                state.clamp_selection();
                vec![Effect::PersistQueue]
            }
            OccurrenceRemoval::Missing => Vec::new(),
        },
        QueueAction::UndoQueueRemoval => {
            if let Some(position) = undo_removal(&mut state.domain) {
                state.ui.selected_index = position;
                state.notify("Queue removal undone", false);
                return vec![Effect::PersistQueue];
            }
            Vec::new()
        }
        QueueAction::MoveSelectedInQueue(delta) => {
            // Reordering a filtered view would move hidden neighbors around
            // invisibly; require the full list.
            if state.ui.visible_indices.is_some() {
                state.notify("Clear the filter (Esc) to reorder", false);
                return Vec::new();
            }
            let len = state.domain.queue.order.len();
            if state.ui.view == View::Queue && len > 1 {
                let from = state.ui.selected_index;
                let to = from.saturating_add_signed(delta as isize).min(len - 1);
                if move_track(&mut state.domain, from, to) {
                    // Keep the cursor on the item that moved.
                    state.ui.selected_index = to;
                    return vec![Effect::PersistQueue];
                }
            }
            Vec::new()
        }
        QueueAction::MoveTrack { from, to } => {
            let len = state.domain.queue.order.len();
            if from < len && to < len && move_track(&mut state.domain, from, to) {
                return vec![Effect::PersistQueue];
            }
            Vec::new()
        }
        QueueAction::ClearQueue => {
            state.ui.confirm = Some(crate::app::state::ConfirmState {
                message: "Clear the entire queue? (y/n)".to_string(),
                action: Box::new(Action::Queue(QueueAction::ClearQueueConfirmed)),
            });
            Vec::new()
        }
        QueueAction::ClearQueueConfirmed => {
            clear(&mut state.domain);
            state.ui.selected_index = 0;
            vec![Effect::PersistQueue]
        }
        QueueAction::QueueExhausted => {
            state.domain.current_track = None;
            state.notify("Queue finished", false);
            Vec::new()
        }
        // Selected-content variants need the full app state and the playlist
        // store to resolve "the selection" into tracks, so the service layer
        // applies them after reduce; there is no pure transition to make here.
        // `QueueLoaded` is a pure report — `client_route` ignores it too.
        QueueAction::QueueLoaded(_)
        | QueueAction::AddSelectedToQueue
        | QueueAction::AddSelectedAsNext
        | QueueAction::LoadSelectedPlaylistIntoQueue
        | QueueAction::AppendSelectedPlaylistToQueue => Vec::new(),
    }
}

/// Append `track` and record the activity event.
fn add_to_queue(domain: &mut DomainState, track: Track) -> Vec<Effect> {
    domain
        .activity
        .push(crate::history::activity::ActivityEvent::new(
            crate::history::activity::ActivityKind::Queued,
            track.title.clone(),
            track.artist.clone(),
        ));
    domain.queue.push(track);
    domain.bump_queue_revision();
    vec![Effect::PersistQueue, Effect::PersistSession]
}

/// Insert `track` after the current position and record the activity event.
fn add_next(domain: &mut DomainState, track: Track) -> Vec<Effect> {
    domain
        .activity
        .push(crate::history::activity::ActivityEvent::new(
            crate::history::activity::ActivityKind::Queued,
            track.title.clone(),
            "Play next",
        ));
    domain.queue.push_next(track);
    domain.bump_queue_revision();
    vec![Effect::PersistQueue, Effect::PersistSession]
}

/// Remove the order position `index`, arming single-level undo.
fn remove_at(domain: &mut DomainState, index: usize) -> bool {
    if let Some(track) = domain.queue.remove_at(index) {
        domain.bump_queue_revision();
        domain.removed_queue_item = Some((index, track));
        true
    } else {
        false
    }
}

/// Remove an exact occurrence only when the queue still matches the captured
/// revision and track (context-menu removals racing queue changes).
fn remove_occurrence(
    domain: &mut DomainState,
    order_index: usize,
    expected_track: &Track,
    expected_revision: u64,
) -> OccurrenceRemoval {
    let still_matches = domain.queue_revision == expected_revision
        && domain
            .queue
            .order
            .get(order_index)
            .and_then(|track_index| domain.queue.tracks.get(*track_index))
            == Some(expected_track);
    if !still_matches {
        return OccurrenceRemoval::Cancelled;
    }
    if remove_at(domain, order_index) {
        OccurrenceRemoval::Removed
    } else {
        OccurrenceRemoval::Missing
    }
}

/// Re-insert the last removed item; returns its position for cursor restore.
fn undo_removal(domain: &mut DomainState) -> Option<usize> {
    let (position, track) = domain.removed_queue_item.take()?;
    domain.queue.insert_at(position, track);
    domain.bump_queue_revision();
    Some(position)
}

/// Reorder `from` to `to`; false when the move is a no-op.
fn move_track(domain: &mut DomainState, from: usize, to: usize) -> bool {
    if from == to {
        return false;
    }
    domain.queue.reorder(from, to);
    domain.bump_queue_revision();
    true
}

/// Empty the queue and drop any pending undo.
fn clear(domain: &mut DomainState) {
    let changed = !domain.queue.tracks.is_empty();
    domain.queue.clear();
    if changed {
        domain.bump_queue_revision();
    }
    domain.removed_queue_item = None;
}
