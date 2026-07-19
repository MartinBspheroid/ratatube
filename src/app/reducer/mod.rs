//! Pure state transitions: `reduce(state, action) -> effects`.
//!
//! The UI never invokes subprocesses directly; reducers return [`Effect`]s
//! that the app layer executes (PRD section 13).

mod effect;
mod history;
mod navigation;
mod playback;
mod playlists;
mod queue;
mod track_context;

pub use effect::Effect;

use crate::app::action::Action;
use crate::app::state::AppState;

/// Apply one action to state and return the side effects to execute.
pub fn reduce(state: &mut AppState, action: Action) -> Vec<Effect> {
    let effects = match action {
        Action::Navigation(action) => navigation::reduce(state, action),
        Action::Playback(action) => playback::reduce(state, action),
        Action::Queue(action) => queue::reduce(state, action),
        Action::Playlists(action) => playlists::reduce(state, action),
        Action::History(action) => history::reduce(state, action),
    };
    state.sync_track_transition(std::time::Instant::now());
    effects
}

#[cfg(test)]
mod tests;
