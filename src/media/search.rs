//! Search state and result handling (PRD section 10.1).

use crate::media::Track;

/// Lifecycle of an asynchronous search.
#[derive(Debug, Clone, Default)]
pub enum SearchState {
    /// No search performed yet.
    #[default]
    Idle,
    /// A search is currently running.
    Searching {
        query: String,
        /// Monotonic id so a superseded search can be discarded (PRD 15).
        generation: u64,
    },
    /// Results are available.
    Results { query: String, tracks: Vec<Track> },
    /// The search failed; message is user-facing.
    Failed { query: String, message: String },
}

impl SearchState {
    /// The query currently shown in the input, if any.
    pub fn query(&self) -> &str {
        match self {
            SearchState::Idle => "",
            SearchState::Searching { query, .. }
            | SearchState::Results { query, .. }
            | SearchState::Failed { query, .. } => query,
        }
    }
}
