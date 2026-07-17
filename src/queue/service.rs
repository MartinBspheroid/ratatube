//! Queue persistence (PRD 10.5): order, current index, shuffle, repeat.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::persistence::json_store;
use crate::persistence::migrations::QUEUE_SCHEMA_VERSION;
use crate::queue::model::Queue;

/// On-disk representation of the queue for session restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedQueue {
    pub schema_version: u32,
    #[serde(flatten)]
    pub queue: Queue,
}

impl From<Queue> for PersistedQueue {
    fn from(queue: Queue) -> Self {
        Self {
            schema_version: QUEUE_SCHEMA_VERSION,
            queue,
        }
    }
}

/// Load the persisted queue, or an empty queue when none exists.
pub fn load(path: &Path) -> Result<Queue> {
    if !path.exists() {
        return Ok(Queue::default());
    }
    let persisted: PersistedQueue = json_store::read(path)?;
    Ok(persisted.queue)
}

/// Persist the queue atomically.
pub fn save(path: &Path, queue: &Queue) -> Result<()> {
    let persisted = PersistedQueue::from(queue.clone());
    json_store::atomic_write(path, &persisted)
}
