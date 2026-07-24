//! Side effects emitted by pure state transitions.

/// Side effects the app layer must perform after a state update.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    RunSearch {
        query: String,
        generation: u64,
    },
    RunExactVideo {
        url: String,
        generation: u64,
    },
    RunImport {
        url: String,
    },
    ResolveAndPlay {
        track_index_in_queue: usize,
    },
    SeekBy(i64),
    SeekTo(f64),
    TogglePause,
    AdjustVolume(i8),
    ToggleMute,
    SetSpeed(f64),
    StopPlayback,
    QuitMpv,
    PersistQueue,
    PersistPlaylists,
    PersistSession,
    /// Apply the settings-menu drafts to configuration and save it.
    PersistUiSettings {
        theme: crate::config::ThemeName,
        icons: crate::config::IconMode,
        resume: crate::config::ResumeMode,
    },
    Exit,
}
