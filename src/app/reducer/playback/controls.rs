//! Playback controls and playback-feel transitions.

use crate::app::action::PlaybackAction;
use crate::app::reducer::Effect;
use crate::app::state::{AppState, DomainState};
use crate::playback::PlaybackEvent;

use super::events::reduce_playback_event;

/// Reduce direct playback controls and playback-feel settings.
pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match action {
        PlaybackAction::PlayPause => play_pause(&mut state.domain),
        PlaybackAction::Stop => vec![Effect::StopPlayback],
        PlaybackAction::SeekForward => vec![Effect::SeekBy(5)],
        PlaybackAction::SeekBackward => vec![Effect::SeekBy(-5)],
        PlaybackAction::SeekForwardLarge => vec![Effect::SeekBy(30)],
        PlaybackAction::SeekBackwardLarge => vec![Effect::SeekBy(-30)],
        PlaybackAction::SeekToFraction(fraction) => {
            if let Some(duration) = state.domain.playback.duration_seconds {
                let target = duration * fraction.clamp(0.0, 1.0);
                return vec![Effect::SeekTo(target)];
            }
            Vec::new()
        }
        PlaybackAction::VolumeUp => vec![Effect::AdjustVolume(2)],
        PlaybackAction::VolumeDown => vec![Effect::AdjustVolume(-2)],
        PlaybackAction::ToggleMute => vec![Effect::ToggleMute],
        PlaybackAction::SpeedUp => speed_step(state, 0.25),
        PlaybackAction::SpeedDown => speed_step(state, -0.25),
        PlaybackAction::SpeedReset => {
            if (state.domain.playback.speed - 1.0).abs() > f64::EPSILON {
                state.notify("Speed 1.00x", false);
                return vec![Effect::SetSpeed(1.0)];
            }
            Vec::new()
        }
        PlaybackAction::CycleSleepTimer => {
            match cycle_sleep_timer(&mut state.domain) {
                Some(minutes) => state.notify(&format!("Sleep timer: {minutes} min"), false),
                None => state.notify("Sleep timer off", false),
            }
            Vec::new()
        }
        PlaybackAction::ToggleRadio => {
            let on = toggle_radio(&mut state.domain);
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
        PlaybackAction::ToggleShuffle => toggle_shuffle(&mut state.domain),
        PlaybackAction::CycleRepeat => cycle_repeat(&mut state.domain),
        PlaybackAction::PlaybackEvent(event) => {
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
        _ => Vec::new(),
    }
}

/// Toggle pause, or arm play-on-load while a session resume is in flight.
fn play_pause(domain: &mut DomainState) -> Vec<Effect> {
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

/// Cycle the sleep timer through 15/30/60 minutes and off.
fn cycle_sleep_timer(domain: &mut DomainState) -> Option<u16> {
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
fn toggle_radio(domain: &mut DomainState) -> bool {
    domain.radio = !domain.radio;
    if !domain.radio {
        domain.radio_operation = None;
    }
    domain.radio
}

fn toggle_shuffle(domain: &mut DomainState) -> Vec<Effect> {
    domain.queue.set_shuffle(!domain.queue.shuffle);
    domain.bump_queue_revision();
    vec![Effect::PersistQueue]
}

fn cycle_repeat(domain: &mut DomainState) -> Vec<Effect> {
    domain.queue.repeat = domain.queue.repeat.next();
    vec![Effect::PersistQueue]
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

/// Step playback speed by `delta`, clamped to 0.5-2.0.
fn speed_step(state: &mut AppState, delta: f64) -> Vec<Effect> {
    let target = (state.domain.playback.speed + delta).clamp(0.5, 2.0);
    if (target - state.domain.playback.speed).abs() > f64::EPSILON {
        state.notify(&format!("Speed {target:.2}x"), false);
        return vec![Effect::SetSpeed(target)];
    }
    Vec::new()
}
