//! Root application state consumed by rendering and reducers.

use crate::app::state::{DomainState, UiState};
use crate::queue::Queue;

/// Root application state: the domain half (future daemon) composed with the
/// UI half (future client) for the current single-process runtime.
#[derive(Default)]
pub struct AppState {
    pub domain: DomainState,
    pub ui: UiState,
}

impl AppState {
    /// Construct running application state with default data.
    pub fn new() -> Self {
        let mut state = Self::default();
        state.ui.running = true;
        state
    }

    /// Attach loaded services' initial data, including a queue restored from disk.
    pub fn with_queue(mut self, queue: Queue) -> Self {
        self.domain.queue = queue;
        self
    }

    /// Invalidate queue occurrence tokens after a membership or order change.
    pub(crate) fn bump_queue_revision(&mut self) {
        self.domain.bump_queue_revision();
    }

    /// Invalidate playlist occurrence tokens after a stored collection change.
    pub(crate) fn bump_playlists_revision(&mut self) {
        self.domain.bump_playlists_revision();
    }
}
