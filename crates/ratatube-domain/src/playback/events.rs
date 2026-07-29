//! Events emitted by the persistent mpv process (PRD section 13).

/// Playback-relevant events observed from mpv's JSON IPC.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackEvent {
    /// mpv acknowledged the IPC connection.
    Connected,
    /// A new file started playing.
    Started,
    /// mpv completed loading a new media file.
    FileLoaded,
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
    /// One throttled window of real audio levels from the astats filter.
    AudioLevels(AudioLevels),
    /// mpv reported a playback error.
    PlaybackError(String),
    /// The mpv process exited or the IPC socket closed.
    Shutdown,
}

/// One measurement window of real audio levels, produced by mpv's `astats`
/// filter and observed over IPC as `af-metadata` (see the level meter in
/// the playback bar).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevels {
    /// Overall RMS level in dBFS (silence is about -90, full scale 0).
    pub rms_db: f32,
    /// Peak sample level in dBFS across channels; peak minus RMS is the
    /// crest factor that spikes on transients.
    pub peak_db: f32,
    /// Mean zero-crossings rate as a fraction of the sample rate (0..0.5);
    /// a cheap spectral-brightness proxy (bass-heavy is low, bright high).
    pub zcr: f32,
}

/// Playback status snapshot derived from events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaybackStatus {
    #[default]
    Idle,
    Playing,
    Paused,
    Stopped,
    Buffering,
}
