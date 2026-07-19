//! Playback controls and playback-feel transitions.

use crate::app::action::{Action, PlaybackAction};
use crate::app::reducer::Effect;
use crate::app::state::AppState;
use crate::playback::PlaybackEvent;

use super::events::reduce_playback_event;

/// Reduce direct playback controls and playback-feel settings.
pub(super) fn reduce(state: &mut AppState, action: PlaybackAction) -> Vec<Effect> {
    match Action::Playback(action) {
        Action::Playback(PlaybackAction::PlayPause) => {
            // While a session resume is in flight, Space means "play it as
            // soon as it's ready" instead of toggling an idle player.
            if let Some(pending) = &mut state.pending_resume
                && !pending.armed
            {
                pending.play_on_load = true;
                return Vec::new();
            }
            state.pending_resume = None;
            return vec![Effect::TogglePause];
        }
        Action::Playback(PlaybackAction::Stop) => return vec![Effect::StopPlayback],
        Action::Playback(PlaybackAction::SeekForward) => return vec![Effect::SeekBy(5)],
        Action::Playback(PlaybackAction::SeekBackward) => return vec![Effect::SeekBy(-5)],
        Action::Playback(PlaybackAction::SeekForwardLarge) => return vec![Effect::SeekBy(30)],
        Action::Playback(PlaybackAction::SeekBackwardLarge) => return vec![Effect::SeekBy(-30)],
        Action::Playback(PlaybackAction::SeekToFraction(fraction)) => {
            if let Some(duration) = state.playback.duration_seconds {
                let target = duration * fraction.clamp(0.0, 1.0);
                return vec![Effect::SeekTo(target)];
            }
        }
        Action::Playback(PlaybackAction::VolumeUp) => return vec![Effect::AdjustVolume(2)],
        Action::Playback(PlaybackAction::VolumeDown) => return vec![Effect::AdjustVolume(-2)],
        Action::Playback(PlaybackAction::ToggleMute) => return vec![Effect::ToggleMute],
        Action::Playback(PlaybackAction::SpeedUp) => return speed_step(state, 0.25),
        Action::Playback(PlaybackAction::SpeedDown) => return speed_step(state, -0.25),
        Action::Playback(PlaybackAction::SpeedReset) => {
            if (state.playback.speed - 1.0).abs() > f64::EPSILON {
                state.notify("Speed 1.00x", false);
                return vec![Effect::SetSpeed(1.0)];
            }
        }
        Action::Playback(PlaybackAction::CycleSleepTimer) => {
            use crate::app::state::SleepTimer;
            let minutes = match state.sleep_timer.map(|t| t.minutes) {
                None => Some(15),
                Some(15) => Some(30),
                Some(30) => Some(60),
                Some(_) => None,
            };
            state.sleep_timer = minutes.map(|m| SleepTimer {
                deadline: std::time::Instant::now()
                    + std::time::Duration::from_secs(u64::from(m) * 60),
                minutes: m,
            });
            match minutes {
                Some(m) => state.notify(&format!("Sleep timer: {m} min"), false),
                None => state.notify("Sleep timer off", false),
            }
        }
        Action::Playback(PlaybackAction::ToggleRadio) => {
            state.radio = !state.radio;
            if !state.radio {
                state.radio_operation = None;
            }
            state.notify(
                if state.radio {
                    "Radio on: the queue will keep itself filled"
                } else {
                    "Radio off"
                },
                false,
            );
        }
        Action::Playback(PlaybackAction::ToggleShuffle) => {
            state.queue.set_shuffle(!state.queue.shuffle);
            state.bump_queue_revision();
            return vec![Effect::PersistQueue];
        }
        Action::Playback(PlaybackAction::CycleRepeat) => {
            state.queue.repeat = state.queue.repeat.next();
            return vec![Effect::PersistQueue];
        }
        Action::Playback(PlaybackAction::PlaybackEvent(event)) => {
            let started = event == PlaybackEvent::Started;
            if started && let Some(track) = &state.current_track {
                state
                    .activity
                    .push(crate::history::activity::ActivityEvent::new(
                        crate::history::activity::ActivityKind::Played,
                        track.title.clone(),
                        track.artist.clone(),
                    ));
            }
            let mut effects = reduce_playback_event(state, event);
            if started {
                effects.push(Effect::PersistSession);
            }
            return effects;
        }

        // --- Queue --------------------------------------------------------
        _ => {}
    }
    Vec::new()
}

/// Step playback speed by `delta`, clamped to 0.5-2.0.
fn speed_step(state: &mut AppState, delta: f64) -> Vec<Effect> {
    let target = (state.playback.speed + delta).clamp(0.5, 2.0);
    if (target - state.playback.speed).abs() > f64::EPSILON {
        state.notify(&format!("Speed {target:.2}x"), false);
        return vec![Effect::SetSpeed(target)];
    }
    Vec::new()
}
