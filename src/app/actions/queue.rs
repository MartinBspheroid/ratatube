//! Queue mutation and queue-selection actions.

use crate::media::Track;

/// An intent that mutates the queue or resolves selected content into it.
#[derive(Debug, Clone)]
pub enum QueueAction {
    AddToQueue(Track),
    AddNext(Track),
    RemoveSelectedFromQueue,
    /// Restore the most recently removed queue item.
    UndoQueueRemoval,
    /// Move the selected queue item up (-1) or down (+1) in play order.
    MoveSelectedInQueue(i32),
    ClearQueue,
    /// Clear the queue after explicit confirmation.
    ClearQueueConfirmed,
    QueueLoaded(Track),
    QueueExhausted,
    /// Resolve the selected track in the app layer, which owns full state and services.
    AddSelectedToQueue,
    /// Resolve the selected track in the app layer and enqueue it next.
    AddSelectedAsNext,
    /// Resolve the selected playlist through the app layer and replace the queue.
    LoadSelectedPlaylistIntoQueue,
    /// Resolve the selected playlist through the app layer and append it to the queue.
    AppendSelectedPlaylistToQueue,
}
