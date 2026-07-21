use crate::app::state::{AppState, PlayingPane, View};
use crate::app::track_context::{TrackContextAction, TrackSource, resolve_track_context};

use super::track;

#[test]
fn ultra_wide_playing_queue_resolves_selected_queue_occurrence() {
    let mut state = AppState::new();
    state.ui.view = View::NowPlaying;
    state.ui.playing_pane = PlayingPane::Queue;
    state.ui.screen_area.width = 170;
    state.domain.current_track = Some(track("playing", "Current track"));
    state.domain.queue.push(track("first", "First queued"));
    state
        .domain
        .queue
        .push(track("selected", "Selected queued"));
    state.ui.selected_index = 1;

    let context = resolve_track_context(&state, None).expect("queue context");

    assert_eq!(context.track.id, "selected");
    assert!(matches!(
        context.source,
        TrackSource::Queue { order_index: 1 }
    ));
    assert!(context.actions.iter().any(|action| matches!(
        action,
        TrackContextAction::RemoveFromQueue { order_index: 1 }
    )));
}
