//! Playback controls and playback-feel transitions.
//!
//! Every entry point takes the payload it needs rather than a
//! `PlaybackAction`, so the family dispatcher in `super` is the only place
//! that enumerates the enum.

use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState};
use crate::playback::PlaybackEvent;

use super::events::reduce_playback_event;

/// Toggle pause, or arm play-on-load while a session resume is in flight.
pub(super) fn play_pause(domain: &mut DomainState) -> Vec<Effect> {
    // While a session resume is in flight, Space means "play it as soon as
    // it's ready" instead of toggling an idle player.
    if let Some(pending) = &mut domain.pending_resume
        && !pending.armed
    {
        pending.play_on_load = true;
        return Vec::new();
    }
    domain.pending_resume = None;
    vec![Effect::TogglePause]
}

/// Stop playback outright.
pub(super) fn stop() -> Vec<Effect> {
    vec![Effect::StopPlayback]
}

/// Seek relative to the playhead.
pub(super) fn seek_by(seconds: i64) -> Vec<Effect> {
    vec![Effect::SeekBy(seconds)]
}

/// Seek to an absolute position, clamped to the start of the track.
pub(super) fn seek_to_seconds(seconds: f64) -> Vec<Effect> {
    vec![Effect::SeekTo(seconds.max(0.0))]
}

/// Seek to a fraction of the duration (a timeline click); an unknown duration
/// has nothing to seek against.
pub(super) fn seek_to_fraction(state: &AppState, fraction: f64) -> Vec<Effect> {
    if let Some(duration) = state.domain.playback.duration_seconds {
        let target = duration * fraction.clamp(0.0, 1.0);
        return vec![Effect::SeekTo(target)];
    }
    Vec::new()
}

/// Change the volume by `delta` percentage points.
pub(super) fn volume_by(delta: i8) -> Vec<Effect> {
    vec![Effect::AdjustVolume(delta)]
}

/// Flip mute.
pub(super) fn toggle_mute() -> Vec<Effect> {
    vec![Effect::ToggleMute]
}

/// Step playback speed by `delta`, clamped to 0.5-2.0.
pub(super) fn speed_step(state: &mut AppState, delta: f64) -> Vec<Effect> {
    let target = (state.domain.playback.speed + delta).clamp(0.5, 2.0);
    if (target - state.domain.playback.speed).abs() > f64::EPSILON {
        state.notify(&format!("Speed {target:.2}x"), false);
        return vec![Effect::SetSpeed(target)];
    }
    Vec::new()
}

/// Restore normal speed, staying quiet when it is already 1.0.
pub(super) fn speed_reset(state: &mut AppState) -> Vec<Effect> {
    if (state.domain.playback.speed - 1.0).abs() > f64::EPSILON {
        state.notify("Speed 1.00x", false);
        return vec![Effect::SetSpeed(1.0)];
    }
    Vec::new()
}

/// Cycle the sleep timer and announce the new setting.
pub(super) fn cycle_sleep_timer(state: &mut AppState) -> Vec<Effect> {
    match advance_sleep_timer(&mut state.domain) {
        Some(minutes) => state.notify(&format!("Sleep timer: {minutes} min"), false),
        None => state.notify("Sleep timer off", false),
    }
    Vec::new()
}

/// Toggle radio mode and announce it; the refill itself is service work.
pub(super) fn toggle_radio(state: &mut AppState) -> Vec<Effect> {
    let on = flip_radio(&mut state.domain);
    state.notify(
        if on {
            "Radio on: the queue will keep itself filled"
        } else {
            "Radio off"
        },
        false,
    );
    Vec::new()
}

/// Flip shuffle and persist the new play order.
pub(super) fn toggle_shuffle(domain: &mut DomainState) -> Vec<Effect> {
    domain.queue.set_shuffle(!domain.queue.shuffle);
    domain.bump_queue_revision();
    vec![Effect::PersistQueue]
}

/// Advance the repeat mode and persist it.
pub(super) fn cycle_repeat(domain: &mut DomainState) -> Vec<Effect> {
    domain.queue.repeat = domain.queue.repeat.next();
    vec![Effect::PersistQueue]
}

/// Fold an mpv event into the snapshot, recording the Played activity and
/// persisting the session when a new track actually started.
pub(super) fn playback_event(state: &mut AppState, event: PlaybackEvent) -> Vec<Effect> {
    let started = event == PlaybackEvent::Started;
    if started {
        record_started_activity(&mut state.domain);
    }
    let mut effects = reduce_playback_event(state, event);
    if started {
        effects.push(Effect::PersistSession);
    }
    effects
}

/// Cycle the sleep timer through 15/30/60 minutes and off.
fn advance_sleep_timer(domain: &mut DomainState) -> Option<u16> {
    use crate::app::state::SleepTimer;
    let minutes = match domain.sleep_timer.map(|t| t.minutes) {
        None => Some(15),
        Some(15) => Some(30),
        Some(30) => Some(60),
        Some(_) => None,
    };
    domain.sleep_timer = minutes.map(|m| SleepTimer {
        deadline: std::time::Instant::now() + std::time::Duration::from_secs(u64::from(m) * 60),
        minutes: m,
    });
    minutes
}

/// Toggle radio mode; disabling drops any in-flight refill.
fn flip_radio(domain: &mut DomainState) -> bool {
    domain.radio = !domain.radio;
    if !domain.radio {
        domain.radio_operation = None;
    }
    domain.radio
}

/// Record a Played activity event for the current track.
fn record_started_activity(domain: &mut DomainState) {
    if let Some(track) = &domain.current_track {
        let event = crate::history::activity::ActivityEvent::new(
            crate::history::activity::ActivityKind::Played,
            track.title.clone(),
            track.artist.clone(),
        );
        domain.activity.push(event);
    }
}
