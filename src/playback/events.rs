//! Events emitted by the persistent mpv process (PRD section 13).

/// Playback-relevant events observed from mpv's JSON IPC.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackEvent {
    /// mpv acknowledged the IPC connection.
    Connected,
    /// A new file started playing.
    Started,
    /// Playback position changed (seconds).
    PositionChanged(f64),
    /// Media duration became known (seconds).
    DurationChanged(f64),
    /// Pause state changed.
    PauseChanged(bool),
    /// Volume changed (0-100).
    VolumeChanged(f64),
    /// Mute state changed.
    MuteChanged(bool),
    /// Playback speed changed (1.0 = normal).
    SpeedChanged(f64),
    /// End of file reached; carries the stop reason ("eof", "error", ...).
    EndFile { reason: String },
    /// mpv reported a playback error.
    PlaybackError(String),
    /// The mpv process exited or the IPC socket closed.
    Shutdown,
}

/// Playback status snapshot derived from events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Idle,
    Playing,
    Paused,
    Stopped,
    Buffering,
}
