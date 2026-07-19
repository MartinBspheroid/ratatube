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
                let queue_position = self
                    .state
                    .queue
                    .order
                    .iter()
                    .position(|&index| self.state.queue.tracks[index].id == track.id)
                    .unwrap_or_else(|| {
                        self.state.queue.push(track.clone());
                        self.state.queue.order.len() - 1
                    });
                self.state.queue.position = Some(queue_position);
                self.state.current_track = Some(track.clone());
                self.state.playback.position_seconds = position_seconds;
                self.state.playback.duration_seconds =
                    track.duration_seconds.map(|value| value as f64);
                self.state.pending_resume = Some(crate::app::state::PendingResume {
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
                    .queue
                    .order
                    .get(queue_position)
                    .and_then(|index| self.state.queue.tracks.get(*index))
                    .is_some_and(|track| track.id == track_id);
                if still_requested && self.config.playback.continue_on_error {
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::NextTrack))
                        .await;
                }
            }
            PlaybackAction::PlaybackEvent(PlaybackEvent::Started) => {
                self.after_track_started(action_tx);
            }
            PlaybackAction::PrefetchResolved { track_id, url, .. } => {
                self.prefetched = Some((track_id, url, Instant::now()));
            }
            PlaybackAction::RadioTracksLoaded { .. } => {
                self.radio_fetching = false;
                self.after_track_started(action_tx);
            }
            PlaybackAction::ToggleRadio if self.state.radio => {
                if self.state.queue.position.is_none() || self.state.current_track.is_none() {
                    let seed = self
                        .state
                        .current_track
                        .as_ref()
                        .map(|track| track.id.clone())
                        .or_else(|| {
                            self.state
                                .queue
                                .order
                                .last()
                                .map(|&index| self.state.queue.tracks[index].id.clone())
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
                let Some(pending) = self.state.pending_resume.as_mut() else {
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
                            if let Some(pending) = self.state.pending_resume.as_mut() {
                                pending.armed = true;
                            }
                        } else {
                            self.state.pending_resume = None;
                        }
                        self.spawn_details_fetch(&track, action_tx);
                        self.spawn_thumbnail_fetch(
                            &track,
                            ThumbnailPurpose::CurrentTrack,
                            action_tx,
                        );
                    }
                    Err(err) => {
                        self.state.pending_resume = None;
                        self.state.notify(&format!("Resume failed: {err}"), true);
                    }
                }
            }
            PlaybackAction::SessionResolveFailed {
                track_id, message, ..
            } => {
                if self
                    .state
                    .pending_resume
                    .as_ref()
                    .is_some_and(|pending| pending.track.id == track_id)
                {
                    self.state.pending_resume = None;
                    if self
                        .state
                        .current_track
                        .as_ref()
                        .map(|track| track.id.as_str())
                        == Some(track_id.as_str())
                    {
                        self.state.current_track = None;
                    }
                    tracing::warn!(%message, "session resume resolve failed");
                    self.state.notify("Couldn't resume the last session", true);
                }
            }
            PlaybackAction::PlaySelected if self.state.view == View::Home => {
                match self.state.home_section {
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
                }
            }
            PlaybackAction::PlaySelected if self.state.view == View::History => {
                if let Some(track) = self.resolve_selected_track() {
                    let _ = action_tx
                        .send(Action::Playback(PlaybackAction::PlayTrack(track)))
                        .await;
                }
            }
            _ => {}
        }
    }
}
