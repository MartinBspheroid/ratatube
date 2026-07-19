//! Playback metadata, chapter, and presentation transitions.

use crate::app::action::{Action, PlaybackAction};
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DetailsStatus, PlayingPane, View};

pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match Action::Playback(action) {
        Action::Playback(PlaybackAction::DetailsStarted {
            operation_id,
            track_id,
        }) => {
            state.details_status = DetailsStatus::Loading {
                operation_id,
                track_id,
            };
        }
        Action::Playback(PlaybackAction::DetailsLoaded {
            operation_id,
            track_id,
            details,
        }) => {
            // Only apply details that still belong to the current track.
            if matches!(
                state.details_status,
                DetailsStatus::Loading {
                    operation_id: active,
                    track_id: ref active_track_id,
                } if active == operation_id && active_track_id == &track_id
            ) && state.current_track.as_ref().map(|t| t.id.as_str()) == Some(track_id.as_str())
            {
                state.current_details = Some(*details);
                state.details_status = DetailsStatus::Idle;
            }
        }
        Action::Playback(PlaybackAction::DetailsFailed {
            operation_id,
            track_id,
            message,
        }) => {
            if matches!(
                state.details_status,
                DetailsStatus::Loading {
                    operation_id: active,
                    track_id: ref active_track_id,
                } if active == operation_id && active_track_id == &track_id
            ) && state.current_track.as_ref().map(|t| t.id.as_str()) == Some(track_id.as_str())
            {
                state.details_status = DetailsStatus::Failed { track_id, message };
            }
        }
        Action::Playback(PlaybackAction::ScrollNowPlaying(delta)) => {
            let next = i32::from(state.now_playing_scroll) + delta;
            state.now_playing_scroll = next.max(0) as u16;
        }
        Action::Playback(PlaybackAction::NextChapter) => {
            let position = state.playback.position_seconds;
            if let Some(chapter) = state
                .chapters()
                .iter()
                .find(|c| c.start_seconds > position + 1.0)
            {
                return vec![Effect::SeekTo(chapter.start_seconds)];
            }
        }
        Action::Playback(PlaybackAction::PreviousChapter) => {
            let chapters = state.chapters();
            if let Some(current) = state.current_chapter_index() {
                let start = chapters[current].start_seconds;
                // Like PreviousTrack: restart the current chapter first,
                // then step back to the one before it.
                let target = if state.playback.position_seconds > start + 3.0 || current == 0 {
                    start
                } else {
                    chapters[current - 1].start_seconds
                };
                return vec![Effect::SeekTo(target)];
            }
        }
        Action::Playback(PlaybackAction::ToggleNowPlayingPane) => {
            state.now_playing_show_description = !state.now_playing_show_description;
            state.now_playing_scroll = 0;
        }
        Action::Playback(PlaybackAction::CyclePlayingPane) => {
            if state.view == View::NowPlaying
                && crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                    == crate::ui::layout::Breakpoint::UltraWide
            {
                state.playing_pane = match state.playing_pane {
                    PlayingPane::Info => PlayingPane::Queue,
                    PlayingPane::Queue => PlayingPane::Info,
                };
                if state.playing_pane == PlayingPane::Queue {
                    state.selected_index = state.queue.position.unwrap_or(0);
                }
                state.reset_list();
            }
        }
        _ => {}
    }
    Vec::new()
}
