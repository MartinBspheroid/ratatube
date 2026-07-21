//! Pure modal transitions for the universal track-context menu and details.

use crate::app::action::NavigationAction;
use crate::app::reducer::Effect;
use crate::app::state::{DomainState, UiState};

/// Reduce context-menu intents that do not require application services.
pub(in crate::app::reducer) fn reduce_track_context(
    ui: &mut UiState,
    domain: &DomainState,
    action: NavigationAction,
) -> Vec<Effect> {
    match action {
        NavigationAction::CloseTrackContext => ui.track_context_menu = None,
        NavigationAction::MoveTrackContext(delta) => {
            if let Some(menu) = &mut ui.track_context_menu {
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
        NavigationAction::ShowTrackDetails(track) => {
            // Existing extended details apply only when they belong to this
            // exact track (mirrors the pre-split AppState helper).
            let details = domain
                .current_track
                .as_ref()
                .is_some_and(|current| current.id == track.id)
                .then(|| domain.current_details.clone())
                .flatten();
            ui.show_track_details(track, details);
        }
        NavigationAction::CloseTrackDetails => ui.track_details_modal = None,
        // Opening needs HistoryService; submission dispatches the selected
        // stable action through existing action domains.
        NavigationAction::OpenTrackContext | NavigationAction::SubmitTrackContext => {}
        _ => unreachable!("non-context action routed to context reducer"),
    }
    Vec::new()
}
