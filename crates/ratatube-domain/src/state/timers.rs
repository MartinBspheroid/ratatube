//! Domain-owned playback timers.

/// An armed sleep timer.
#[derive(Debug, Clone, Copy)]
pub struct SleepTimer {
    pub deadline: std::time::Instant,
    /// The duration originally chosen, for cycling and display.
    pub minutes: u16,
}

impl SleepTimer {
    /// Whole minutes left, rounded up; 0 when due.
    pub fn remaining_minutes(&self) -> u64 {
        let now = std::time::Instant::now();
        self.deadline
            .saturating_duration_since(now)
            .as_secs()
            .div_ceil(60)
    }
}
