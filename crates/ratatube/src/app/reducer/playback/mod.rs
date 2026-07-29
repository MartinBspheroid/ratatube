//! Playback reducer routing by sub-responsibility.
//!
//! This match is the single enumeration of [`PlaybackAction`] on the reducer
//! side and it is wildcard-free on purpose: a new variant cannot compile until
//! it is given an owner here, so it can never silently reduce to nothing (the
//! failure mode 348e200 shipped). The sub-reducers take the payload they need
//! instead of the whole enum, which is what makes a catch-all unnecessary
//! rather than merely hidden.

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
        // Transport, seeking, volume, and playback-feel controls.
        PlaybackAction::PlayPause => controls::play_pause(&mut state.domain),
        PlaybackAction::Stop => controls::stop(),
        PlaybackAction::SeekForward => controls::seek_by(5),
        PlaybackAction::SeekBackward => controls::seek_by(-5),
        PlaybackAction::SeekForwardLarge => controls::seek_by(30),
        PlaybackAction::SeekBackwardLarge => controls::seek_by(-30),
        PlaybackAction::SeekBy(seconds) => controls::seek_by(seconds),
        PlaybackAction::SeekToSeconds(seconds) => controls::seek_to_seconds(seconds),
        PlaybackAction::SeekToFraction(fraction) => controls::seek_to_fraction(state, fraction),
        PlaybackAction::VolumeUp => controls::volume_by(2),
        PlaybackAction::VolumeDown => controls::volume_by(-2),
        PlaybackAction::VolumeBy(delta) => controls::volume_by(delta),
        PlaybackAction::ToggleMute => controls::toggle_mute(),
        PlaybackAction::ToggleShuffle => controls::toggle_shuffle(&mut state.domain),
        PlaybackAction::CycleRepeat => controls::cycle_repeat(&mut state.domain),
        PlaybackAction::SpeedUp => controls::speed_step(state, 0.25),
        PlaybackAction::SpeedDown => controls::speed_step(state, -0.25),
        PlaybackAction::SpeedReset => controls::speed_reset(state),
        PlaybackAction::CycleSleepTimer => controls::cycle_sleep_timer(state),
        PlaybackAction::ToggleRadio => controls::toggle_radio(state),
        PlaybackAction::PlaybackEvent(event) => controls::playback_event(state, event),

        // Queue selection and stream resolution.
        PlaybackAction::PlaySelected => queue::play_selected(state),
        PlaybackAction::PlayQueuePosition(position) => queue::play_queue_position(state, position),
        PlaybackAction::PlayTrack(track) => queue::play_track(&mut state.domain, track),
        PlaybackAction::NextTrack => queue::next_track(&mut state.domain),
        PlaybackAction::PreviousTrack => queue::previous_track(&mut state.domain),
        PlaybackAction::SessionStreamResolved { track_id, .. } => {
            queue::session_stream_resolved(state, &track_id)
        }
        PlaybackAction::PlaybackResolveStarted { operation_id, .. } => {
            queue::resolve_started(state, operation_id)
        }
        PlaybackAction::PlaybackResolved {
            operation_id,
            queue_position,
            track_id,
            ..
        } => queue::resolved(state, operation_id, queue_position, &track_id),
        PlaybackAction::PlaybackResolveFailed {
            operation_id,
            message,
            ..
        } => queue::resolve_failed(state, operation_id, &message),

        // Extended metadata and chapter navigation.
        PlaybackAction::DetailsStarted {
            operation_id,
            track_id,
        } => media::details_started(&mut state.domain, operation_id, track_id),
        PlaybackAction::DetailsLoaded {
            operation_id,
            track_id,
            details,
        } => media::details_loaded(&mut state.domain, operation_id, track_id, *details),
        PlaybackAction::DetailsFailed {
            operation_id,
            track_id,
            message,
        } => media::details_failed(&mut state.domain, operation_id, track_id, message),
        PlaybackAction::NextChapter => media::next_chapter(&state.domain),
        PlaybackAction::PreviousChapter => media::previous_chapter(&state.domain),

        // Mixes and radio refills.
        PlaybackAction::MixLoaded { title, tracks, .. } => radio::mix_loaded(state, title, tracks),
        PlaybackAction::RadioRefillStarted { operation_id } => {
            radio::refill_started(&mut state.domain, operation_id)
        }
        PlaybackAction::RadioTracksLoaded {
            operation_id,
            tracks,
        } => radio::tracks_loaded(state, operation_id, tracks),

        // No domain transition to make: the app layer owns the pending-session
        // resume flow (`ResumeTrack`, `SessionResolveFailed`), the UI reducer
        // owns thumbnail bytes, prefetch is a service-side cache, and
        // `RepeatChanged` only reports a change made outside the reducer.
        PlaybackAction::ResumeTrack { .. }
        | PlaybackAction::SessionResolveFailed { .. }
        | PlaybackAction::ThumbnailLoaded { .. }
        | PlaybackAction::SearchThumbnailLoaded { .. }
        | PlaybackAction::PrefetchResolved { .. }
        | PlaybackAction::RepeatChanged(_) => Vec::new(),
    }
}
