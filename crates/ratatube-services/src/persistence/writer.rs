//! Ordered, coalescing owner for blocking persistence operations.

use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};

use ratatube_domain::error::{AppError, Result};

type WriteJob = Box<dyn FnOnce() -> Result<()> + Send + 'static>;

enum Message {
    Write {
        key: String,
        description: String,
        job: WriteJob,
    },
    Flush(oneshot::Sender<()>),
}

/// Result of one durable write, suitable for user-visible reporting.
#[derive(Debug)]
pub struct WriteOutcome {
    pub key: String,
    pub description: String,
    pub result: Result<()>,
}

/// Serializes writes, coalesces queued writes by key, and supports flush.
pub struct PersistenceWriter {
    tx: mpsc::UnboundedSender<Message>,
    task: tokio::task::JoinHandle<()>,
}

impl PersistenceWriter {
    /// Start a writer and return its outcome stream.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<WriteOutcome>) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (outcome_tx, outcome_rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                match message {
                    Message::Write {
                        key,
                        description,
                        job,
                    } => {
                        let mut order = vec![key.clone()];
                        let mut pending = HashMap::from([(key, (description, job))]);
                        let mut barrier = None;
                        while let Ok(next) = rx.try_recv() {
                            match next {
                                Message::Write {
                                    key,
                                    description,
                                    job,
                                } => {
                                    if !pending.contains_key(&key) {
                                        order.push(key.clone());
                                    }
                                    pending.insert(key, (description, job));
                                }
                                Message::Flush(sender) => {
                                    barrier = Some(sender);
                                    break;
                                }
                            }
                        }
                        for key in order {
                            let Some((description, job)) = pending.remove(&key) else {
                                continue;
                            };
                            let result =
                                tokio::task::spawn_blocking(job)
                                    .await
                                    .unwrap_or_else(|err| {
                                        Err(AppError::Process {
                                            command: "persistence writer".to_string(),
                                            message: err.to_string(),
                                        })
                                    });
                            let _ = outcome_tx.send(WriteOutcome {
                                key,
                                description,
                                result,
                            });
                        }
                        if let Some(sender) = barrier {
                            let _ = sender.send(());
                        }
                    }
                    Message::Flush(sender) => {
                        let _ = sender.send(());
                    }
                }
            }
        });
        (Self { tx, task }, outcome_rx)
    }

    /// Queue a write. A newer queued write with the same key replaces it.
    pub fn submit(
        &self,
        key: impl Into<String>,
        description: impl Into<String>,
        job: impl FnOnce() -> Result<()> + Send + 'static,
    ) -> Result<()> {
        self.tx
            .send(Message::Write {
                key: key.into(),
                description: description.into(),
                job: Box::new(job),
            })
            .map_err(|_| AppError::Storage {
                path: "persistence-writer".into(),
                message: "writer has stopped".to_string(),
            })
    }

    /// Wait until every write submitted before this call has completed.
    pub async fn flush(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Message::Flush(tx))
            .map_err(|_| AppError::Storage {
                path: "persistence-writer".into(),
                message: "writer has stopped".to_string(),
            })?;
        rx.await.map_err(|_| AppError::Storage {
            path: "persistence-writer".into(),
            message: "flush acknowledgement was lost".to_string(),
        })
    }

    /// Flush and stop the owned worker within a bounded deadline.
    pub async fn shutdown(self) -> Result<()> {
        self.flush().await?;
        drop(self.tx);
        self.task.await.map_err(|err| AppError::Process {
            command: "persistence writer".to_string(),
            message: err.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[tokio::test]
    async fn preserves_key_order_and_coalesces_queued_replacements() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (writer, mut outcomes) = PersistenceWriter::new();
        for (key, value) in [("queue", 1), ("history", 2), ("queue", 3)] {
            let calls = calls.clone();
            writer
                .submit(key, key, move || {
                    calls.lock().expect("calls").push(value);
                    Ok(())
                })
                .expect("submit");
        }
        writer.flush().await.expect("flush");

        assert_eq!(*calls.lock().expect("calls"), vec![3, 2]);
        assert!(outcomes.recv().await.expect("queue outcome").result.is_ok());
        assert!(
            outcomes
                .recv()
                .await
                .expect("history outcome")
                .result
                .is_ok()
        );
        writer.shutdown().await.expect("shutdown");
    }

    #[tokio::test]
    async fn reports_failures_and_flushes_successful_writes() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("saved");
        let saved_path = path.clone();
        let (writer, mut outcomes) = PersistenceWriter::new();
        writer
            .submit("failure", "history", || {
                Err(AppError::Config("read only".to_string()))
            })
            .expect("submit failure");
        writer
            .submit("success", "queue", move || {
                std::fs::write(saved_path, b"durable")?;
                Ok(())
            })
            .expect("submit success");

        writer.flush().await.expect("flush");

        assert!(
            outcomes
                .recv()
                .await
                .expect("failure outcome")
                .result
                .is_err()
        );
        assert!(
            outcomes
                .recv()
                .await
                .expect("success outcome")
                .result
                .is_ok()
        );
        assert_eq!(std::fs::read(path).expect("flushed file"), b"durable");
        writer.shutdown().await.expect("shutdown");
    }
}
