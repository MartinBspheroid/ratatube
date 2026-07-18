//! Queue persistence (PRD 10.5): order, current index, shuffle, repeat.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::persistence::json_store;
use crate::persistence::migrations::{QUEUE_SCHEMA_VERSION, migrate_in_place};
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
    migrate_in_place(path, QUEUE_SCHEMA_VERSION)?;
    let persisted: PersistedQueue = json_store::read(path)?;
    if persisted.schema_version > QUEUE_SCHEMA_VERSION {
        return Err(AppError::UnsupportedSchema {
            path: path.to_path_buf(),
            found: persisted.schema_version,
            supported: QUEUE_SCHEMA_VERSION,
        });
    }
    if let Err(error) = persisted.queue.validate() {
        let backup_result = json_store::preserve_backup(path);
        tracing::warn!(
            ?path,
            ?error,
            ?backup_result,
            "invalid queue; backup preserved"
        );
        return Err(AppError::MalformedData(path.to_path_buf()));
    }
    Ok(persisted.queue)
}

/// Persist the queue atomically.
pub fn save(path: &Path, queue: &Queue) -> Result<()> {
    let persisted = PersistedQueue::from(queue.clone());
    json_store::atomic_write(path, &persisted)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::error::AppError;
    use crate::media::Track;

    fn valid_queue() -> Queue {
        let mut queue = Queue::default();
        queue.push(Track::new("a", "A", "Artist"));
        queue
    }

    #[test]
    fn load_rejects_invalid_indices_and_preserves_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("queue.json");
        let mut raw = serde_json::to_value(PersistedQueue::from(valid_queue())).expect("serialize");
        raw["order"] = serde_json::json!([99]);
        fs::write(&path, serde_json::to_vec_pretty(&raw).expect("json")).expect("write");

        let result = load(&path);

        assert!(
            matches!(result, Err(AppError::MalformedData(ref error_path)) if error_path == &path)
        );
        assert!(path.with_extension("json.bak").exists());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path).expect("original"))
                .expect("parse original")["order"],
            serde_json::json!([99])
        );
    }

    #[test]
    fn load_rejects_future_schema_without_rewriting_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("queue.json");
        let mut raw = serde_json::to_value(PersistedQueue::from(valid_queue())).expect("serialize");
        raw["schemaVersion"] = serde_json::json!(99);
        let original = serde_json::to_vec_pretty(&raw).expect("json");
        fs::write(&path, &original).expect("write");

        let result = load(&path);

        assert!(matches!(
            result,
            Err(AppError::UnsupportedSchema {
                found: 99,
                supported: QUEUE_SCHEMA_VERSION,
                ..
            })
        ));
        assert_eq!(fs::read(&path).expect("unchanged"), original);
    }

    #[test]
    fn load_migrates_pre_versioned_queue() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("queue.json");
        let mut raw = serde_json::to_value(PersistedQueue::from(valid_queue())).expect("serialize");
        raw.as_object_mut().expect("object").remove("schemaVersion");
        fs::write(&path, serde_json::to_vec_pretty(&raw).expect("json")).expect("write");

        let loaded = load(&path).expect("migrated queue");

        assert_eq!(loaded.tracks.len(), 1);
        let migrated: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("migrated file")).expect("parse");
        assert_eq!(migrated["schemaVersion"], QUEUE_SCHEMA_VERSION);
        assert!(path.with_extension("pre-migration.bak").exists());
    }
}
