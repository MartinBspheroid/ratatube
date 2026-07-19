use crate::app::state::{AppState, PlayingPane, View};
use crate::app::track_context::{TrackContextAction, TrackSource, resolve_track_context};

use super::track;

#[test]
fn ultra_wide_playing_queue_resolves_selected_queue_occurrence() {
    let mut state = AppState::new();
    state.view = View::NowPlaying;
    state.playing_pane = PlayingPane::Queue;
    state.screen_area.width = 170;
    state.current_track = Some(track("playing", "Current track"));
    state.queue.push(track("first", "First queued"));
    state.queue.push(track("selected", "Selected queued"));
    state.selected_index = 1;

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
