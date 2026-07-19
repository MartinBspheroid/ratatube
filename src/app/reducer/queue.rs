//! Queue state transitions.

use crate::app::action::{Action, QueueAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, View};

pub(super) fn reduce(state: &mut AppState, action: QueueAction) -> Vec<Effect> {
    match Action::Queue(action) {
        Action::Queue(QueueAction::AddToQueue(track)) => {
            state
                .activity
                .push(crate::history::activity::ActivityEvent::new(
                    crate::history::activity::ActivityKind::Queued,
                    track.title.clone(),
                    track.artist.clone(),
                ));
            state.queue.push(track);
            state.notify("Added to queue", false);
            return vec![Effect::PersistQueue, Effect::PersistSession];
        }
        Action::Queue(QueueAction::AddNext(track)) => {
            state
                .activity
                .push(crate::history::activity::ActivityEvent::new(
                    crate::history::activity::ActivityKind::Queued,
                    track.title.clone(),
                    "Play next",
                ));
            state.queue.push_next(track);
            state.notify("Will play next", false);
            return vec![Effect::PersistQueue, Effect::PersistSession];
        }
        Action::Queue(QueueAction::RemoveSelectedFromQueue) => {
            if state.view == View::Queue {
                let real = state.resolve_index(state.selected_index);
                if let Some(track) = state.queue.remove_at(real) {
                    state.removed_queue_item = Some((real, track));
                    state.notify("Removed from queue — u to undo", false);
                }
                // The filter indices refresh next loop; drop them now so the
                // stale mapping can't resolve a second removal wrongly.
                if let Some(indices) = &mut state.visible_indices {
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
        Action::Queue(QueueAction::UndoQueueRemoval) => {
            if let Some((position, track)) = state.removed_queue_item.take() {
                state.queue.insert_at(position, track);
                state.selected_index = position;
                state.notify("Queue removal undone", false);
                return vec![Effect::PersistQueue];
            }
        }
        Action::Queue(QueueAction::MoveSelectedInQueue(delta)) => {
            // Reordering a filtered view would move hidden neighbors around
            // invisibly; require the full list.
            if state.visible_indices.is_some() {
                state.notify("Clear the filter (Esc) to reorder", false);
                return Vec::new();
            }
            let len = state.queue.order.len();
            if state.view == View::Queue && len > 1 {
                let from = state.selected_index;
                let to = from.saturating_add_signed(delta as isize).min(len - 1);
                if from != to {
                    state.queue.reorder(from, to);
                    // Keep the cursor on the item that moved.
                    state.selected_index = to;
                    return vec![Effect::PersistQueue];
                }
            }
        }
        Action::Queue(QueueAction::ClearQueue) => {
            state.confirm = Some(crate::app::state::ConfirmState {
                message: "Clear the entire queue? (y/n)".to_string(),
                action: Box::new(Action::Queue(QueueAction::ClearQueueConfirmed)),
            });
        }
        Action::Queue(QueueAction::ClearQueueConfirmed) => {
            state.queue.clear();
            state.removed_queue_item = None;
            state.selected_index = 0;
            return vec![Effect::PersistQueue];
        }
        Action::Queue(QueueAction::QueueExhausted) => {
            state.current_track = None;
            state.notify("Queue finished", false);
        }
        _ => {}
    }
    Vec::new()
}
