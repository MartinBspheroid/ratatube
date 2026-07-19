//! Playback reducer routing by sub-responsibility.

mod controls;
mod events;
mod media;
mod queue;
mod radio;

use crate::app::action::PlaybackAction;
use crate::app::reducer::Effect;
use crate::app::state::AppState;

/// Route a playback action to its focused reducer.
pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match action {
        action @ (PlaybackAction::PlayPause
        | PlaybackAction::Stop
        | PlaybackAction::SeekForward
        | PlaybackAction::SeekBackward
        | PlaybackAction::SeekForwardLarge
        | PlaybackAction::SeekBackwardLarge
        | PlaybackAction::SeekToFraction(_)
        | PlaybackAction::VolumeUp
        | PlaybackAction::VolumeDown
        | PlaybackAction::ToggleMute
        | PlaybackAction::ToggleShuffle
        | PlaybackAction::CycleRepeat
        | PlaybackAction::PlaybackEvent(_)
        | PlaybackAction::SpeedUp
        | PlaybackAction::SpeedDown
        | PlaybackAction::SpeedReset
        | PlaybackAction::CycleSleepTimer
        | PlaybackAction::ToggleRadio) => controls::reduce(state, action),
        action @ (PlaybackAction::PlaySelected
        | PlaybackAction::PlayTrack(_)
        | PlaybackAction::ResumeTrack { .. }
        | PlaybackAction::SessionStreamResolved { .. }
        | PlaybackAction::PlaybackResolveStarted { .. }
        | PlaybackAction::PlaybackResolved { .. }
        | PlaybackAction::PlaybackResolveFailed { .. }
        | PlaybackAction::NextTrack
        | PlaybackAction::PreviousTrack) => queue::reduce(state, action),
        action @ (PlaybackAction::DetailsStarted { .. }
        | PlaybackAction::DetailsLoaded { .. }
        | PlaybackAction::DetailsFailed { .. }
        | PlaybackAction::ScrollNowPlaying(_)
        | PlaybackAction::NextChapter
        | PlaybackAction::PreviousChapter
        | PlaybackAction::ToggleNowPlayingPane
        | PlaybackAction::CyclePlayingPane) => media::reduce(state, action),
        action @ (PlaybackAction::MixLoaded { .. }
        | PlaybackAction::RadioRefillStarted { .. }
        | PlaybackAction::RadioTracksLoaded { .. }) => radio::reduce(state, action),
        PlaybackAction::SessionResolveFailed { .. }
        | PlaybackAction::ThumbnailLoaded { .. }
        | PlaybackAction::SearchThumbnailLoaded { .. }
        | PlaybackAction::PrefetchResolved { .. }
        | PlaybackAction::RepeatChanged(_) => Vec::new(),
    }
}
