//! Queue state transitions.

use crate::app::action::{Action, QueueAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, View};

/// Reduce queue-domain state transitions.
pub(super) fn reduce(state: &mut AppState, action: QueueAction) -> Vec<Effect> {
    match Action::Queue(action) {
        Action::Queue(QueueAction::AddToQueue(track)) => {
            state
                .domain
                .activity
                .push(crate::history::activity::ActivityEvent::new(
                    crate::history::activity::ActivityKind::Queued,
                    track.title.clone(),
                    track.artist.clone(),
                ));
            state.domain.queue.push(track);
            state.bump_queue_revision();
            state.notify("Added to queue", false);
            return vec![Effect::PersistQueue, Effect::PersistSession];
        }
        Action::Queue(QueueAction::AddNext(track)) => {
            state
                .domain
                .activity
                .push(crate::history::activity::ActivityEvent::new(
                    crate::history::activity::ActivityKind::Queued,
                    track.title.clone(),
                    "Play next",
                ));
            state.domain.queue.push_next(track);
            state.bump_queue_revision();
            state.notify("Will play next", false);
            return vec![Effect::PersistQueue, Effect::PersistSession];
        }
        Action::Queue(QueueAction::RemoveSelectedFromQueue) => {
            if state.ui.view == View::Queue {
                let real = state.resolve_index(state.ui.selected_index);
                if let Some(track) = state.domain.queue.remove_at(real) {
                    state.bump_queue_revision();
                    state.domain.removed_queue_item = Some((real, track));
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
                return vec![Effect::PersistQueue];
            }
        }
        Action::Queue(QueueAction::RemoveTrackOccurrence {
            order_index,
            expected_track,
            expected_revision,
        }) => {
            let still_matches = state.domain.queue_revision == expected_revision
                && state
                    .domain
                    .queue
                    .order
                    .get(order_index)
                    .and_then(|track_index| state.domain.queue.tracks.get(*track_index))
                    == Some(&expected_track);
            if !still_matches {
                state.notify("Queue changed; removal cancelled", true);
                return Vec::new();
            }
            if let Some(track) = state.domain.queue.remove_at(order_index) {
                state.bump_queue_revision();
                state.domain.removed_queue_item = Some((order_index, track));
                state.notify("Removed from queue — u to undo", false);
                state.ui.visible_indices = None;
                state.clamp_selection();
                return vec![Effect::PersistQueue];
            }
        }
        Action::Queue(QueueAction::UndoQueueRemoval) => {
            if let Some((position, track)) = state.domain.removed_queue_item.take() {
                state.domain.queue.insert_at(position, track);
                state.bump_queue_revision();
                state.ui.selected_index = position;
                state.notify("Queue removal undone", false);
                return vec![Effect::PersistQueue];
            }
        }
        Action::Queue(QueueAction::MoveSelectedInQueue(delta)) => {
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
                if from != to {
                    state.domain.queue.reorder(from, to);
                    state.bump_queue_revision();
                    // Keep the cursor on the item that moved.
                    state.ui.selected_index = to;
                    return vec![Effect::PersistQueue];
                }
            }
        }
        Action::Queue(QueueAction::ClearQueue) => {
            state.ui.confirm = Some(crate::app::state::ConfirmState {
                message: "Clear the entire queue? (y/n)".to_string(),
                action: Box::new(Action::Queue(QueueAction::ClearQueueConfirmed)),
            });
        }
        Action::Queue(QueueAction::ClearQueueConfirmed) => {
            let changed = !state.domain.queue.tracks.is_empty();
            state.domain.queue.clear();
            if changed {
                state.bump_queue_revision();
            }
            state.domain.removed_queue_item = None;
            state.ui.selected_index = 0;
            return vec![Effect::PersistQueue];
        }
        Action::Queue(QueueAction::QueueExhausted) => {
            state.domain.current_track = None;
            state.notify("Queue finished", false);
        }
        _ => {}
    }
    Vec::new()
}
