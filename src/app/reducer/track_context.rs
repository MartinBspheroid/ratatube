//! Pure state transitions for universal track-context modal intents.

use crate::app::action::NavigationAction;
use crate::app::reducer::Effect;
use crate::app::state::AppState;

/// Reduce context-menu intents that do not require application services.
pub(super) fn reduce(state: &mut AppState, action: NavigationAction) -> Vec<Effect> {
    match action {
        NavigationAction::CloseTrackContext => state.track_context_menu = None,
        NavigationAction::MoveTrackContext(delta) => {
            if let Some(menu) = &mut state.track_context_menu {
                let len = menu.context.actions.len();
                if len > 0 {
                    menu.selected = (menu.selected as i32 + delta).rem_euclid(len as i32) as usize;
                }
            }
        }
        // Opening needs HistoryService; Task 3 consumes submit and dispatches
        // the selected stable action through existing action domains.
        NavigationAction::OpenTrackContext | NavigationAction::SubmitTrackContext => {}
        _ => unreachable!("non-context action routed to context reducer"),
    }
    Vec::new()
}
