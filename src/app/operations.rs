//! Ownership and cancellation for asynchronous application operations.

use std::collections::HashMap;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Monotonic identity attached to an asynchronous operation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

/// Independently supersedable operation families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperationKind {
    Playback,
    Import,
    Radio,
    Details,
    Thumbnail,
    SearchThumbnail,
    Search,
    Prefetch,
    Mix,
    Session,
    PlaybackRecovery,
    ExternalCommand,
}

/// Identity and cooperative cancellation handle returned when work starts.
#[derive(Debug, Clone)]
pub struct OperationTicket {
    id: OperationId,
    cancellation: CancellationToken,
}

impl OperationTicket {
    /// Return the identity that completion actions must carry.
    pub fn id(&self) -> OperationId {
        self.id
    }

    /// Return the token tasks should select against while awaiting work.
    pub fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }
}

#[derive(Debug)]
struct RunningOperation {
    id: OperationId,
    cancellation: CancellationToken,
    handle: Option<JoinHandle<()>>,
}

/// Owns all detached application work and invalidates superseded results.
#[derive(Debug, Default)]
pub struct OperationRegistry {
    next_id: u64,
    running: HashMap<OperationKind, RunningOperation>,
}

impl OperationRegistry {
    /// Cancel prior work of this kind and allocate a fresh operation ticket.
    pub fn start(&mut self, kind: OperationKind) -> OperationTicket {
        self.cancel(kind);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let id = OperationId(self.next_id);
        let cancellation = CancellationToken::new();
        self.running.insert(
            kind,
            RunningOperation {
                id,
                cancellation: cancellation.clone(),
                handle: None,
            },
        );
        OperationTicket { id, cancellation }
    }

    /// Attach the spawned task so shutdown and supersession can abort it.
    pub fn attach(&mut self, kind: OperationKind, id: OperationId, handle: JoinHandle<()>) {
        match self.running.get_mut(&kind) {
            Some(operation) if operation.id == id => operation.handle = Some(handle),
            _ => handle.abort(),
        }
    }

    /// Return whether a completion still belongs to the active operation.
    pub fn is_current(&self, kind: OperationKind, id: OperationId) -> bool {
        self.running
            .get(&kind)
            .is_some_and(|operation| operation.id == id)
    }

    /// Accept and remove a current completion; stale completions return false.
    pub fn complete(&mut self, kind: OperationKind, id: OperationId) -> bool {
        if !self.is_current(kind, id) {
            return false;
        }
        self.running.remove(&kind);
        true
    }

    /// Cancel and abort the active operation of one kind.
    pub fn cancel(&mut self, kind: OperationKind) {
        if let Some(operation) = self.running.remove(&kind) {
            operation.cancellation.cancel();
            if let Some(handle) = operation.handle {
                handle.abort();
            }
        }
    }

    /// Cancel and abort every operation owned by the application.
    pub fn cancel_all(&mut self) {
        for (_, operation) in self.running.drain() {
            operation.cancellation.cancel();
            if let Some(handle) = operation.handle {
                handle.abort();
            }
        }
    }

    /// Cancel every operation and wait up to `timeout` for task teardown.
    pub async fn shutdown(&mut self, timeout: Duration) {
        let mut handles = Vec::new();
        for (_, operation) in self.running.drain() {
            operation.cancellation.cancel();
            if let Some(handle) = operation.handle {
                handle.abort();
                handles.push(handle);
            }
        }
        let wait = async {
            for handle in handles {
                let _ = handle.await;
            }
        };
        let _ = tokio::time::timeout(timeout, wait).await;
    }
}

impl Drop for OperationRegistry {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{OperationKind, OperationRegistry};

    #[tokio::test]
    async fn starting_same_kind_cancels_and_invalidates_previous_operation() {
        let mut registry = OperationRegistry::default();
        let first = registry.start(OperationKind::Playback);
        let second = registry.start(OperationKind::Playback);

        assert!(first.cancellation().is_cancelled());
        assert!(!registry.is_current(OperationKind::Playback, first.id()));
        assert!(registry.is_current(OperationKind::Playback, second.id()));
    }

    #[tokio::test]
    async fn delayed_operation_does_not_block_independent_actions() {
        let mut registry = OperationRegistry::default();
        let ticket = registry.start(OperationKind::Playback);
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let delayed_tx = tx.clone();
        let cancellation = ticket.cancellation().clone();
        registry.attach(
            OperationKind::Playback,
            ticket.id(),
            tokio::spawn(async move {
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(30)) => {
                        let _ = delayed_tx.send("resolved").await;
                    }
                    () = cancellation.cancelled() => {}
                }
            }),
        );

        tx.send("quit").await.expect("send independent action");
        let received = tokio::time::timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect("action dispatch stayed responsive");
        assert_eq!(received, Some("quit"));
    }

    #[tokio::test]
    async fn cancel_all_aborts_every_owned_task() {
        let mut registry = OperationRegistry::default();
        let playback = registry.start(OperationKind::Playback);
        let details = registry.start(OperationKind::Details);

        registry.cancel_all();

        assert!(playback.cancellation().is_cancelled());
        assert!(details.cancellation().is_cancelled());
        assert!(!registry.is_current(OperationKind::Playback, playback.id()));
        assert!(!registry.is_current(OperationKind::Details, details.id()));
    }
}
