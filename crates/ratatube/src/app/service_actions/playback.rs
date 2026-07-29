//! Playback actions requiring mpv, yt-dlp, history, or whole-state coordination.

use std::time::Instant;

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaybackAction, PlaylistAction};
use crate::app::reducer::Effect;
use crate::app::state::{HomeSection, View};
use crate::app::thumbnails::ThumbnailPurpose;
use crate::playback::PlaybackEvent;

impl App {
    /// Coordinate playback actions that require mpv, yt-dlp, history, or full state.
    pub(super) async fn handle_playback_service(
        &mut self,
        action: PlaybackAction,
        action_tx: &mpsc::Sender<Action>,
    ) {
        match action {
            PlaybackAction::ResumeTrack {
                track,
                position_seconds,
            } => {
                let existing_position = self
                    .state
                    .domain
                    .queue
                    .order
                    .iter()
                    .position(|&index| self.state.domain.queue.tracks[index].id == track.id);
                let queue_position = match existing_position {
                    Some(position) => position,
                    None => {
                        self.state.domain.queue.push(track.clone());
                        self.state.bump_queue_revision();
                        self.state.domain.queue.order.len() - 1
                    }
                };
                self.state.domain.queue.position = Some(queue_position);
                self.state.domain.current_track = Some(track.clone());
                self.state.domain.playback.position_seconds = position_seconds;
                self.state.domain.playback.duration_seconds =
                    track.duration_seconds.map(|value| value as f64);
                self.state.domain.pending_resume = Some(crate::app::state::PendingResume {
                    track: track.clone(),
                    position_seconds,
                    armed: false,
                    play_on_load: true,
                });
                self.execute(vec![Effect::PersistQueue], action_tx).await;
                self.spawn_pending_resume_resolution(track, action_tx);
            }
            PlaybackAction::PlaybackResolved { track_id, url, .. } => {
                let Some(track) = self
                    .state
                    .domain
                    .current_track
                    .clone()
                    .filter(|track| track.id == track_id)
                else {
                    return;
                };
                let loaded = match self.playback.as_mut() {
                    Some(playback) => playback.queue_load(&url, &track.title),
                    None => {
                        self.state.notify("mpv not running", true);
                        return;
                    }
                };
                if let Err(err) = loaded {
                    self.state.notify(&format!("mpv: {err}"), true);
                } else {
                    self.spawn_details_fetch(&track, action_tx);
                    self.spawn_thumbnail_fetch(&track, ThumbnailPurpose::CurrentTrack, action_tx);
                }
            }
            PlaybackAction::PlaybackResolveFailed {
                queue_position,
                track_id,
                ..
            } => {
                let still_requested = self
                    .state
                    .domain
                    .queue
                    .order
                    .get(queue_position)
                    .and_then(|index| self.state.domain.queue.tracks.get(*index))
                    .is_some_and(|track| track.id == track_id);
                if still_requested && self.config.playback.continue_on_error {
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::NextTrack))
                        .await;
                }
            }
            PlaybackAction::PlaybackEvent(event) => {
                if matches!(event, PlaybackEvent::Started) {
                    self.after_track_started(action_tx);
                }
            }
            PlaybackAction::PrefetchResolved { track_id, url, .. } => {
                self.prefetched = Some((track_id, url, Instant::now()));
            }
            PlaybackAction::RadioTracksLoaded { .. } => {
                self.radio_fetching = false;
                // The reducer already appended; prefetch the fresh next track.
                self.after_track_started(action_tx);
            }
            PlaybackAction::ToggleRadio => {
                // Radio switched on with a finished queue: refill immediately.
                if self.state.domain.radio
                    && (self.state.domain.queue.position.is_none()
                        || self.state.domain.current_track.is_none())
                {
                    let seed = self
                        .state
                        .domain
                        .current_track
                        .as_ref()
                        .map(|track| track.id.clone())
                        .or_else(|| {
                            self.state
                                .domain
                                .queue
                                .order
                                .last()
                                .map(|&index| self.state.domain.queue.tracks[index].id.clone())
                        })
                        .or_else(|| {
                            self.history
                                .as_ref()
                                .and_then(|history| history.entries().first())
                                .map(|entry| entry.track_id.clone())
                        });
                    if let Some(seed) = seed {
                        self.spawn_radio_refill(seed, action_tx);
                    } else {
                        self.state
                            .notify("Radio needs a seed — play something first", false);
                    }
                }
            }
            PlaybackAction::SessionStreamResolved { track_id, url, .. } => {
                let Some(pending) = self.state.domain.pending_resume.as_mut() else {
                    return;
                };
                if pending.track.id != track_id {
                    return;
                }
                let paused = !pending.play_on_load;
                let title = pending.track.title.clone();
                let position = pending.position_seconds;
                let track = pending.track.clone();
                let loaded = match self.playback.as_mut() {
                    Some(playback) => playback.queue_load_at(&url, &title, Some(position), paused),
                    None => return,
                };
                match loaded {
                    Ok(()) => {
                        if paused {
                            if let Some(pending) = self.state.domain.pending_resume.as_mut() {
                                pending.armed = true;
                            }
                        } else {
                            self.state.domain.pending_resume = None;
                        }
                        self.spawn_details_fetch(&track, action_tx);
                        self.spawn_thumbnail_fetch(
                            &track,
                            ThumbnailPurpose::CurrentTrack,
                            action_tx,
                        );
                    }
                    Err(err) => {
                        self.state.domain.pending_resume = None;
                        self.state.notify(&format!("Resume failed: {err}"), true);
                    }
                }
            }
            PlaybackAction::SessionResolveFailed {
                track_id, message, ..
            } => {
                if self
                    .state
                    .domain
                    .pending_resume
                    .as_ref()
                    .is_some_and(|pending| pending.track.id == track_id)
                {
                    self.state.domain.pending_resume = None;
                    if self
                        .state
                        .domain
                        .current_track
                        .as_ref()
                        .map(|track| track.id.as_str())
                        == Some(track_id.as_str())
                    {
                        self.state.domain.current_track = None;
                    }
                    tracing::warn!(%message, "session resume resolve failed");
                    self.state.notify("Couldn't resume the last session", true);
                }
            }
            PlaybackAction::PlaySelected => self.play_selected(action_tx).await,

            // Transport, seeking, volume, and supervised completions are fully
            // reduced or executed as effects; no service coordination needed.
            PlaybackAction::PlayPause
            | PlaybackAction::Stop
            | PlaybackAction::PlayTrack(_)
            | PlaybackAction::PlaybackResolveStarted { .. }
            | PlaybackAction::NextTrack
            | PlaybackAction::PreviousTrack
            | PlaybackAction::SeekForward
            | PlaybackAction::SeekBackward
            | PlaybackAction::SeekForwardLarge
            | PlaybackAction::SeekBackwardLarge
            | PlaybackAction::SeekBy(_)
            | PlaybackAction::SeekToSeconds(_)
            | PlaybackAction::SeekToFraction(_)
            | PlaybackAction::PlayQueuePosition(_)
            | PlaybackAction::VolumeUp
            | PlaybackAction::VolumeDown
            | PlaybackAction::VolumeBy(_)
            | PlaybackAction::ToggleMute
            | PlaybackAction::ToggleShuffle
            | PlaybackAction::CycleRepeat
            | PlaybackAction::DetailsStarted { .. }
            | PlaybackAction::DetailsLoaded { .. }
            | PlaybackAction::DetailsFailed { .. }
            | PlaybackAction::ThumbnailLoaded { .. }
            | PlaybackAction::SearchThumbnailLoaded { .. }
            | PlaybackAction::NextChapter
            | PlaybackAction::PreviousChapter
            | PlaybackAction::SpeedUp
            | PlaybackAction::SpeedDown
            | PlaybackAction::SpeedReset
            | PlaybackAction::CycleSleepTimer
            | PlaybackAction::RadioRefillStarted { .. }
            | PlaybackAction::MixLoaded { .. }
            | PlaybackAction::RepeatChanged(_) => {}
        }
    }

    /// Resolve "play the selection" for the views whose selection is not a
    /// plain queue position; other views reduce it directly.
    async fn play_selected(&mut self, action_tx: &mpsc::Sender<Action>) {
        match self.state.ui.view {
            View::Home => match self.state.ui.home_section {
                HomeSection::Resume => {
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::PlayPause))
                        .await;
                }
                HomeSection::Recent => {
                    if let Some(track) = self.resolve_selected_track() {
                        let _ = action_tx
                            .send(Action::Playback(PlaybackAction::PlayTrack(track)))
                            .await;
                    }
                }
                HomeSection::Playlists => {
                    if let Some(id) = self.selected_playlist_id() {
                        let _ = action_tx
                            .send(Action::Playlists(PlaylistAction::LoadPlaylistIntoQueue(id)))
                            .await;
                    }
                }
            },
            View::History => {
                if let Some(track) = self.resolve_selected_track() {
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::PlayTrack(track)))
                        .await;
                }
            }
            View::Search
            | View::Queue
            | View::Playlists
            | View::PlaylistDetail
            | View::NowPlaying
            | View::Channel
            | View::Help => {}
        }
    }
}
