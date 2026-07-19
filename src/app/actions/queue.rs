//! Queue mutation and queue-selection actions.

use crate::media::Track;

/// An intent that mutates the queue or resolves selected content into it.
#[derive(Debug, Clone)]
pub enum QueueAction {
    AddToQueue(Track),
    AddNext(Track),
    RemoveSelectedFromQueue,
    UndoQueueRemoval,
    /// Move the selected queue item up or down in play order.
    MoveSelectedInQueue(i32),
    ClearQueue,
    ClearQueueConfirmed,
    QueueLoaded(Track),
    QueueExhausted,
    AddSelectedToQueue,
    AddSelectedAsNext,
    LoadSelectedPlaylistIntoQueue,
    AppendSelectedPlaylistToQueue,
}
