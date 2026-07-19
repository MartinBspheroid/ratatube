//! State model for the one-shot final-window track-title transition.

use std::time::{Duration, Instant};

/// Remaining playback time at which the transition becomes eligible.
pub const TRANSITION_THRESHOLD_SECONDS: f64 = 15.0;
/// Active playback time used to animate between track titles.
pub const TRANSITION_DURATION: Duration = Duration::from_secs(4);

/// Playback and queue facts consumed by [`TrackTransitionState::update`].
#[derive(Debug, Clone, Copy)]
pub struct TransitionInput<'a> {
    /// Stable identity of the current track.
    pub track_id: Option<&'a str>,
    /// Known duration minus current playback position.
    pub remaining_seconds: Option<f64>,
    /// Whether playback is actively progressing.
    pub playing: bool,
    /// Whether the queue currently resolves a concrete next track.
    pub has_next: bool,
}

/// One-shot transition timing owned by application state.
#[derive(Debug, Clone, Default)]
pub struct TrackTransitionState {
    track_id: Option<String>,
    fired: bool,
    active: bool,
    playing: bool,
    accumulated: Duration,
    resumed_at: Option<Instant>,
}

impl TrackTransitionState {
    /// Reconcile the transition with a playback snapshot and queue state.
    pub fn update(&mut self, input: TransitionInput<'_>, now: Instant) {
        if self.track_id.as_deref() != input.track_id {
            self.reset(input.track_id);
        }
        self.capture_elapsed(now);

        let remaining = input
            .remaining_seconds
            .filter(|value| value.is_finite() && *value >= 0.0);
        let inside_window = remaining.is_some_and(|value| value <= TRANSITION_THRESHOLD_SECONDS);

        if !input.has_next || !inside_window {
            self.active = false;
        } else if !self.fired && input.playing && input.track_id.is_some() {
            self.fired = true;
            self.active = true;
            self.accumulated = Duration::ZERO;
        }

        self.playing = input.playing;
        self.resumed_at = (self.active && self.playing).then_some(now);
    }

    /// Return normalized animation progress, retaining the final position.
    pub fn progress(&self, now: Instant) -> Option<f64> {
        if !self.active {
            return None;
        }
        let elapsed = self.accumulated
            + self.resumed_at.map_or(Duration::ZERO, |started| {
                now.saturating_duration_since(started)
            });
        Some((elapsed.as_secs_f64() / TRANSITION_DURATION.as_secs_f64()).clamp(0.0, 1.0))
    }

    fn capture_elapsed(&mut self, now: Instant) {
        if let Some(started) = self.resumed_at.take() {
            self.accumulated += now.saturating_duration_since(started);
        }
    }

    fn reset(&mut self, track_id: Option<&str>) {
        self.track_id = track_id.map(str::to_owned);
        self.fired = false;
        self.active = false;
        self.playing = false;
        self.accumulated = Duration::ZERO;
        self.resumed_at = None;
    }
}
