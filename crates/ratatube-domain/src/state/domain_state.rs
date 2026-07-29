//! Domain half of the application state: playback, queue, playlists,
//! history-adjacent data, and external-tool health. This is the state the
//! future daemon owns; it must stay free of terminal/rendering types.

use crate::media::Track;
use crate::media::import::InputKind;
use crate::media::search::SearchState;
use crate::operations::OperationId;
use crate::playback::{PlaybackEvent, PlaybackSnapshot, TrackTransitionState};
use crate::playlists::Playlist;
use crate::queue::Queue;
use crate::state::{DetailsStatus, ImportState, OperationStatus, PendingResume, SleepTimer};

/// Domain state: everything the service half owns. Reducers mutate it; the
/// UI half only reads it.
#[derive(Default)]
pub struct DomainState {
    // Search
    pub search: SearchState,
    /// Classified kind of the current search input (URL vs query).
    pub input_kind: Option<InputKind>,
    /// Monotonic generation used to discard superseded searches (PRD 15).
    pub search_generation: u64,

    // Queue
    pub queue: Queue,
    /// Monotonic identity for queue membership and play-order occurrences.
    pub queue_revision: u64,
    /// Single-level undo for accidental queue deletion.
    pub removed_queue_item: Option<(usize, Track)>,

    // Playlists
    pub playlists: Vec<Playlist>,
    /// Monotonic identity for stored playlist membership and track ordering.
    pub playlists_revision: u64,

    /// Dedicated channel-browser lifecycle, absent outside that flow.
    pub channel: Option<crate::state::channel::ChannelState>,
    /// Playlist import flow state (fetching, review, failed).
    pub import: Option<ImportState>,

    // Playback
    pub playback: PlaybackSnapshot,
    pub current_track: Option<Track>,
    /// One-shot final-window title transition timing.
    pub track_transition: TrackTransitionState,
    /// Unique generation for each accepted playback load.
    pub playback_occurrence: u64,
    /// Occurrence for which mpv has emitted the genuine `file-loaded` boundary.
    pub playback_loaded_occurrence: Option<u64>,
    /// Occurrence owning the current position snapshot.
    pub position_occurrence: Option<u64>,
    /// Occurrence owning the current duration snapshot.
    pub duration_occurrence: Option<u64>,
    /// Resolution state for the track requested by the queue cursor.
    pub playback_resolution: OperationStatus,
    /// Extended metadata for the current track, loaded in the background.
    pub current_details: Option<crate::media::TrackDetails>,
    /// Truthful status for metadata shown in the now-playing view.
    pub details_status: DetailsStatus,

    // Radio / timers
    /// Radio mode: when the queue runs low, append tracks from YouTube's mix
    /// for the last played track.
    pub radio: bool,
    /// Active radio refill, used to reject late results after disable.
    pub radio_operation: Option<OperationId>,
    /// Sleep timer that stops playback at its deadline.
    pub sleep_timer: Option<SleepTimer>,

    // Session / history-adjacent
    /// Previous-session track preloaded for one-key resume.
    pub pending_resume: Option<PendingResume>,
    /// Persisted, bounded product activity shown on Home and Playing.
    pub activity: crate::history::activity::ActivityLog,
    /// Persisted per-track resume positions shown on Home.
    pub resume_points: crate::resume::ResumePoints,

    // External-tool health
    pub mpv_ready: bool,
    pub yt_dlp_ready: bool,
}

impl DomainState {
    /// Invalidate queue occurrence tokens after a membership or order change.
    pub fn bump_queue_revision(&mut self) {
        self.queue_revision = self.queue_revision.wrapping_add(1);
    }

    /// Invalidate playlist occurrence tokens after a stored collection change.
    pub fn bump_playlists_revision(&mut self) {
        self.playlists_revision = self.playlists_revision.wrapping_add(1);
    }
}

impl DomainState {
    /// Reconcile final-window timing with the current playback and queue facts.
    pub fn sync_track_transition(&mut self, now: std::time::Instant) {
        let occurrence = (self.current_track.is_some() && self.playback_occurrence != 0)
            .then_some(self.playback_occurrence);
        let timing_is_current = occurrence.is_some()
            && self.position_occurrence == occurrence
            && self.duration_occurrence == occurrence;
        let remaining_seconds = timing_is_current
            .then_some(self.playback.duration_seconds)
            .flatten()
            .filter(|duration| duration.is_finite() && *duration >= 0.0)
            .zip(
                self.playback
                    .position_seconds
                    .is_finite()
                    .then_some(self.playback.position_seconds),
            )
            .map(|(duration, position)| (duration - position).max(0.0));
        self.track_transition.update(
            crate::playback::TransitionInput {
                occurrence,
                remaining_seconds,
                playing: self.playback.status == crate::playback::PlaybackStatus::Playing,
                has_next: self.queue.effective_next().is_some(),
            },
            now,
        );
    }

    /// Start a distinct playback occurrence and invalidate all prior timing.
    pub fn begin_playback_occurrence(&mut self) {
        self.playback_occurrence = self.playback_occurrence.wrapping_add(1).max(1);
        self.playback_loaded_occurrence = None;
        self.position_occurrence = None;
        self.duration_occurrence = None;
        self.playback.position_seconds = 0.0;
        self.playback.duration_seconds = None;
    }

    /// Mark mpv's genuine media-load boundary for the accepted occurrence.
    pub fn mark_file_loaded(&mut self) {
        if self.playback_occurrence != 0 {
            self.playback_loaded_occurrence = Some(self.playback_occurrence);
        }
    }

    /// Record position only after the active load's `file-loaded` boundary.
    ///
    /// Non-finite and negative values are dropped: `position_seconds` is a bare
    /// `f64`, so a NaN or infinity would serialize to JSON `null` and poison
    /// every snapshot the daemon broadcasts.
    pub fn record_position(&mut self, position: f64) {
        if !position.is_finite() || position < 0.0 {
            return;
        }
        if self.playback_loaded_occurrence == Some(self.playback_occurrence) {
            self.playback.position_seconds = position;
            self.position_occurrence = Some(self.playback_occurrence);
        }
    }

    /// Record duration only after the active load's `file-loaded` boundary.
    ///
    /// Non-finite and negative values are dropped for the same reason as in
    /// `record_position`.
    pub fn record_duration(&mut self, duration: f64) {
        if !duration.is_finite() || duration < 0.0 {
            return;
        }
        if self.playback_loaded_occurrence == Some(self.playback_occurrence) {
            self.playback.duration_seconds = Some(duration);
            self.duration_occurrence = Some(self.playback_occurrence);
        }
    }

    /// Keep playlist presentation and selection order newest-updated first.
    pub fn sort_playlists_by_updated(&mut self) {
        self.playlists
            .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    }

    /// Whether the now-playing bar should render (PRD section 8).
    pub fn has_now_playing(&self) -> bool {
        self.current_track.is_some()
    }

    /// Chapters of the current track, whether uploader-set or parsed from a tracklist.
    pub fn chapters(&self) -> &[crate::media::Chapter] {
        self.current_details
            .as_ref()
            .map_or(&[], |details| details.chapters.as_slice())
    }

    /// Index of the chapter the playhead is currently inside.
    pub fn current_chapter_index(&self) -> Option<usize> {
        crate::media::chapter_at(self.chapters(), self.playback.position_seconds)
    }
}

impl DomainState {
    /// Mirror a playback event into the snapshot (subset of controller logic).
    pub fn playback_status_from(&mut self, event: &PlaybackEvent) {
        use crate::playback::PlaybackStatus;
        match event {
            PlaybackEvent::Started => {
                self.playback.status = PlaybackStatus::Playing;
            }
            PlaybackEvent::FileLoaded => self.mark_file_loaded(),
            PlaybackEvent::PositionChanged(position) => self.record_position(*position),
            PlaybackEvent::DurationChanged(duration) => self.record_duration(*duration),
            PlaybackEvent::PauseChanged(paused) => {
                self.playback.status = if *paused {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Playing
                };
                if *paused {
                    self.playback.audio_levels = None;
                }
            }
            PlaybackEvent::VolumeChanged(v) => {
                self.playback.volume = (*v).clamp(0.0, 100.0) as u8;
            }
            PlaybackEvent::MuteChanged(m) => self.playback.muted = *m,
            PlaybackEvent::SpeedChanged(s) => self.playback.speed = *s,
            PlaybackEvent::AudioLevels(levels) => self.playback.audio_levels = Some(*levels),
            PlaybackEvent::EndFile { .. } => {
                self.playback.status = PlaybackStatus::Stopped;
                self.playback.audio_levels = None;
            }
            PlaybackEvent::PlaybackError(_) | PlaybackEvent::Shutdown => {
                self.playback.status = PlaybackStatus::Idle;
                self.playback.audio_levels = None;
            }
            PlaybackEvent::Connected => self.mpv_ready = true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DomainState;

    fn loaded_state() -> DomainState {
        let mut state = DomainState::default();
        state.begin_playback_occurrence();
        state.mark_file_loaded();
        state
    }

    #[test]
    fn record_position_rejects_non_finite_and_negative_values() {
        let mut state = loaded_state();
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            state.record_position(value);
            assert_eq!(state.playback.position_seconds, 0.0);
            assert_eq!(state.position_occurrence, None);
        }
        state.record_position(12.5);
        assert_eq!(state.playback.position_seconds, 12.5);
        assert_eq!(state.position_occurrence, Some(state.playback_occurrence));
        state.record_position(f64::NAN);
        assert_eq!(state.playback.position_seconds, 12.5);
    }

    #[test]
    fn record_duration_rejects_non_finite_and_negative_values() {
        let mut state = loaded_state();
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            state.record_duration(value);
            assert_eq!(state.playback.duration_seconds, None);
            assert_eq!(state.duration_occurrence, None);
        }
        state.record_duration(200.0);
        assert_eq!(state.playback.duration_seconds, Some(200.0));
        assert_eq!(state.duration_occurrence, Some(state.playback_occurrence));
        state.record_duration(f64::INFINITY);
        assert_eq!(state.playback.duration_seconds, Some(200.0));
    }
}
