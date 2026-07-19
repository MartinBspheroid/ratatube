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
                    let selected = menu.selected % len;
                    let delta = (i64::from(delta)).rem_euclid(len as i64) as usize;
                    menu.selected = if selected >= len - delta {
                        selected - (len - delta)
                    } else {
                        selected + delta
                    };
                }
            }
        }
        NavigationAction::ShowTrackDetails(track) => state.show_track_details(track),
        NavigationAction::CloseTrackDetails => state.track_details_modal = None,
        // Opening needs HistoryService; submission dispatches the selected
        // stable action through existing action domains.
        NavigationAction::OpenTrackContext | NavigationAction::SubmitTrackContext => {}
        _ => unreachable!("non-context action routed to context reducer"),
    }
    Vec::new()
}
