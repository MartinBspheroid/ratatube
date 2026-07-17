//! Pure state transitions: `reduce(state, action) -> effects`.
//!
//! The UI never invokes subprocesses directly; reducers return [`Effect`]s
//! that the app layer executes (PRD section 13).

use crate::app::action::Action;
use crate::app::state::{AppState, Focus, Notification, View};
use crate::media::search::SearchState;
use crate::playback::PlaybackEvent;
use crate::queue::PreviousOutcome;

/// Side effects the app layer must perform after a state update.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    RunSearch { query: String, generation: u64 },
    RunImport { url: String },
    ResolveAndPlay { track_index_in_queue: usize },
    SeekBy(i64),
    SeekTo(f64),
    TogglePause,
    AdjustVolume(i8),
    ToggleMute,
    StopPlayback,
    QuitMpv,
    PersistQueue,
    PersistPlaylists,
    Exit,
}

/// Apply one action to the state, returning effects to execute.
pub fn reduce(state: &mut AppState, action: Action) -> Vec<Effect> {
    match action {
        Action::Navigate(view) => {
            state.view = view;
            state.selected_index = 0;
            state.focus = Focus::Content;
            state.reset_list();
        }
        Action::NextView => {
            return reduce(state, Action::Navigate(state.view.next_tab()));
        }
        Action::PreviousView => {
            return reduce(state, Action::Navigate(state.view.prev_tab()));
        }
        Action::CycleHomeSection(delta) => {
            if state.view == View::Home {
                state.home_section = state.home_section.cycled(delta);
                state.selected_index = 0;
                state.reset_list();
            }
        }
        Action::Quit => {
            state.running = false;
            return vec![Effect::PersistQueue, Effect::QuitMpv, Effect::Exit];
        }

        // --- Search input -------------------------------------------------
        Action::SearchInput(c) => {
            if state.focus == Focus::SearchInput {
                state.search_input.push(c);
            }
        }
        Action::SearchBackspace => {
            if state.focus == Focus::SearchInput {
                state.search_input.pop();
            }
        }
        Action::ClearSearch => {
            state.search_input.clear();
            state.search = SearchState::Idle;
        }
        Action::SubmitSearch(query) => {
            if query.trim().is_empty() {
                return Vec::new();
            }
            state.search_generation += 1;
            let generation = state.search_generation;
            state.search = SearchState::Searching {
                query: query.clone(),
                generation,
            };
            state.focus = Focus::Content;
            return vec![Effect::RunSearch { query, generation }];
        }
        Action::SearchCompleted { generation, tracks } => {
            // Discard results from superseded searches (PRD 15).
            if generation == state.search_generation {
                let query = state.search.query().to_string();
                if tracks.is_empty() {
                    state.notify("No results", false);
                }
                state.search = SearchState::Results { query, tracks };
                state.selected_index = 0;
            }
        }
        Action::SearchFailed {
            generation,
            message,
        } => {
            if generation == state.search_generation {
                let query = state.search.query().to_string();
                state.search = SearchState::Failed { query, message };
            }
        }

        // --- Selection ----------------------------------------------------
        Action::SelectNext => {
            let len = state.active_list_len();
            if len > 0 {
                state.selected_index = (state.selected_index + 1).min(len - 1);
            }
        }
        Action::SelectPrevious => {
            state.selected_index = state.selected_index.saturating_sub(1);
        }

        // --- Playback -----------------------------------------------------
        Action::PlayPause => {
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
        Action::Stop => return vec![Effect::StopPlayback],
        Action::SeekForward => return vec![Effect::SeekBy(5)],
        Action::SeekBackward => return vec![Effect::SeekBy(-5)],
        Action::SeekForwardLarge => return vec![Effect::SeekBy(30)],
        Action::SeekBackwardLarge => return vec![Effect::SeekBy(-30)],
        Action::SeekToFraction(fraction) => {
            if let Some(duration) = state.playback.duration_seconds {
                let target = duration * fraction.clamp(0.0, 1.0);
                return vec![Effect::SeekTo(target)];
            }
        }
        Action::VolumeUp => return vec![Effect::AdjustVolume(2)],
        Action::VolumeDown => return vec![Effect::AdjustVolume(-2)],
        Action::ToggleMute => return vec![Effect::ToggleMute],
        Action::ToggleShuffle => {
            state.queue.set_shuffle(!state.queue.shuffle);
            return vec![Effect::PersistQueue];
        }
        Action::CycleRepeat => {
            state.queue.repeat = state.queue.repeat.next();
            return vec![Effect::PersistQueue];
        }
        Action::PlaybackEvent(event) => {
            return reduce_playback_event(state, event);
        }

        // --- Queue --------------------------------------------------------
        Action::AddToQueue(track) => {
            state.queue.push(track);
            state.notify("Added to queue", false);
            return vec![Effect::PersistQueue];
        }
        Action::AddNext(track) => {
            state.queue.push_next(track);
            state.notify("Will play next", false);
            return vec![Effect::PersistQueue];
        }
        Action::RemoveSelectedFromQueue => {
            if state.view == View::Queue {
                state.queue.remove_at(state.selected_index);
                state.clamp_selection();
                return vec![Effect::PersistQueue];
            }
        }
        Action::MoveSelectedInQueue(delta) => {
            let len = state.queue.order.len();
            if state.view == View::Queue && len > 1 {
                let from = state.selected_index;
                let to = from.saturating_add_signed(delta as isize).min(len - 1);
                if from != to {
                    state.queue.reorder(from, to);
                    // Keep the cursor on the item that moved.
                    state.selected_index = to;
                    return vec![Effect::PersistQueue];
                }
            }
        }
        Action::ClearQueue => {
            state.queue.clear();
            state.selected_index = 0;
            return vec![Effect::PersistQueue];
        }
        Action::PlayTrack(track) => {
            state.queue.push(track);
            let pos = state.queue.order.len() - 1;
            state.queue.position = Some(pos);
            state.current_track = state.queue.current().cloned();
            state.current_details = None;
            state.thumbnail = None;
            state.now_playing_scroll = 0;
            return vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: pos,
                },
                Effect::PersistQueue,
            ];
        }
        Action::PlaySelected => {
            if let Some(track) = selected_track(state) {
                return reduce(state, Action::PlayTrack(track));
            }
        }
        Action::NextTrack => {
            if let Some(track) = state.queue.advance().cloned() {
                state.current_track = Some(track);
                state.current_details = None;
                state.thumbnail = None;
                state.now_playing_scroll = 0;
                let pos = state.queue.position.unwrap_or(0);
                return vec![
                    Effect::ResolveAndPlay {
                        track_index_in_queue: pos,
                    },
                    Effect::PersistQueue,
                ];
            }
        }
        Action::PreviousTrack => {
            let position = state.playback.position_seconds as u64;
            match state.queue.previous(position, 5) {
                PreviousOutcome::RestartCurrent => return vec![Effect::SeekTo(0.0)],
                PreviousOutcome::PlayPrevious => {
                    state.current_track = state.queue.current().cloned();
                    state.current_details = None;
                    state.thumbnail = None;
                    state.now_playing_scroll = 0;
                    let pos = state.queue.position.unwrap_or(0);
                    return vec![
                        Effect::ResolveAndPlay {
                            track_index_in_queue: pos,
                        },
                        Effect::PersistQueue,
                    ];
                }
            }
        }
        Action::DetailsLoaded { track_id, details } => {
            // Only apply details that still belong to the current track.
            if state.current_track.as_ref().map(|t| t.id.as_str()) == Some(track_id.as_str()) {
                state.current_details = Some(*details);
            }
        }
        Action::ScrollNowPlaying(delta) => {
            let next = i32::from(state.now_playing_scroll) + delta;
            state.now_playing_scroll = next.max(0) as u16;
        }
        Action::QueueExhausted => {
            state.current_track = None;
            state.notify("Queue finished", false);
        }

        // --- Modal UI ----------------------------------------------------
        Action::OpenPrompt(purpose) => {
            state.prompt = Some(crate::app::state::PromptState {
                purpose,
                buffer: String::new(),
            });
        }
        Action::PromptInput(c) => {
            if let Some(prompt) = &mut state.prompt {
                prompt.buffer.push(c);
            }
        }
        Action::PromptBackspace => {
            if let Some(prompt) = &mut state.prompt {
                prompt.buffer.pop();
            }
        }
        Action::PromptCancel => state.prompt = None,
        Action::ConfirmNo => state.confirm = None,
        // PromptSubmit / ConfirmYes are resolved by the app layer, which
        // knows the services required by each purpose.

        // --- Playlists ----------------------------------------------------
        Action::OpenPlaylistDetail => {
            if state.view == View::Playlists && !state.playlists.is_empty() {
                state.selected_playlist = Some(state.selected_index);
                state.view = View::PlaylistDetail;
                state.selected_index = 0;
            }
        }
        Action::PlaylistSaved(playlist) => {
            match state.playlists.iter().position(|p| p.id == playlist.id) {
                Some(i) => state.playlists[i] = playlist,
                None => state.playlists.push(playlist),
            }
            state
                .playlists
                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        Action::DeletePlaylist(id) => {
            state.confirm = Some(crate::app::state::ConfirmState {
                message: "Delete this playlist? (local file only, y/n)".to_string(),
                action: Box::new(Action::DeletePlaylistConfirmed(id)),
            });
        }

        // --- Import --------------------------------------------------------
        Action::StartImport(url) => {
            state.import = Some(crate::app::state::ImportState::Fetching { url: url.clone() });
            state.prompt = None;
            return vec![Effect::RunImport { url }];
        }
        Action::ImportCompleted {
            url,
            title,
            remote_id,
            tracks,
            skipped,
        } => {
            let total_entries = tracks.len() + skipped;
            let (playlist, summary) = crate::playlists::import::build_import(
                title,
                url,
                remote_id,
                total_entries,
                tracks,
                skipped,
            );
            state.import = Some(crate::app::state::ImportState::Review {
                summary,
                playlist: Box::new(playlist),
            });
        }
        Action::ImportFailed { message, .. } => {
            state.import = None;
            state.notify(&format!("Import failed: {message}"), true);
        }
        Action::CancelImport => state.import = None,
        // ConfirmImport is executed by the app layer (persists the playlist).

        // --- Notifications -------------------------------------------------
        Action::Notify(message) => state.notify(&message, false),
        Action::DismissNotification => state.notification = None,

        // Remaining actions are handled by the app layer (services), not the
        // pure reducer.
        _ => {}
    }
    Vec::new()
}

/// Translate mpv events into follow-up actions (autoplay next on EOF, etc.).
fn reduce_playback_event(state: &mut AppState, event: PlaybackEvent) -> Vec<Effect> {
    let status_before = state.playback.status;
    state.playback_status_from(&event);
    match event {
        PlaybackEvent::EndFile { ref reason } if reason == "eof" => {
            // Natural completion: advance the queue.
            reduce(state, Action::NextTrack)
        }
        PlaybackEvent::EndFile { ref reason } if reason == "error" => {
            state.notify("Playback failed; skipping track", true);
            reduce(state, Action::NextTrack)
        }
        PlaybackEvent::Shutdown => {
            state.mpv_ready = false;
            if status_before != state.playback.status {
                state.notify("mpv disconnected", true);
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Track selected in the active view, if the view lists tracks.
fn selected_track(state: &AppState) -> Option<crate::media::Track> {
    match state.view {
        View::Search => match &state.search {
            SearchState::Results { tracks, .. } => tracks.get(state.selected_index).cloned(),
            _ => None,
        },
        View::Queue => state
            .queue
            .order
            .get(state.selected_index)
            .map(|&i| state.queue.tracks[i].clone()),
        View::PlaylistDetail => state
            .selected_playlist
            .and_then(|i| state.playlists.get(i))
            .and_then(|p| p.tracks.get(state.selected_index))
            .map(crate::media::Track::from),
        _ => None,
    }
}

impl AppState {
    /// Record a transient notification.
    pub fn notify(&mut self, message: &str, is_error: bool) {
        self.notification = Some(Notification {
            message: message.to_string(),
            is_error,
        });
    }

    /// Mirror a playback event into the snapshot (subset of controller logic).
    fn playback_status_from(&mut self, event: &PlaybackEvent) {
        use crate::playback::PlaybackStatus;
        match event {
            PlaybackEvent::Started => self.playback.status = PlaybackStatus::Playing,
            PlaybackEvent::PositionChanged(p) => self.playback.position_seconds = *p,
            PlaybackEvent::DurationChanged(d) => self.playback.duration_seconds = Some(*d),
            PlaybackEvent::PauseChanged(paused) => {
                self.playback.status = if *paused {
                    PlaybackStatus::Paused
                } else {
                    PlaybackStatus::Playing
                };
            }
            PlaybackEvent::VolumeChanged(v) => {
                self.playback.volume = (*v).clamp(0.0, 100.0) as u8;
            }
            PlaybackEvent::MuteChanged(m) => self.playback.muted = *m,
            PlaybackEvent::EndFile { .. } => self.playback.status = PlaybackStatus::Stopped,
            PlaybackEvent::PlaybackError(_) | PlaybackEvent::Shutdown => {
                self.playback.status = PlaybackStatus::Idle;
            }
            PlaybackEvent::Connected => self.mpv_ready = true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Track;

    fn track(id: &str) -> Track {
        Track::new(id, id, "artist")
    }

    #[test]
    fn superseded_search_results_are_discarded() {
        let mut state = AppState::new();
        reduce(&mut state, Action::SubmitSearch("first".to_string()));
        reduce(&mut state, Action::SubmitSearch("second".to_string()));
        let stale_generation = state.search_generation - 1;
        reduce(
            &mut state,
            Action::SearchCompleted {
                generation: stale_generation,
                tracks: vec![track("stale")],
            },
        );
        assert!(matches!(state.search, SearchState::Searching { .. }));
    }

    #[test]
    fn quit_persists_and_exits() {
        let mut state = AppState::new();
        let effects = reduce(&mut state, Action::Quit);
        assert!(!state.running);
        assert!(effects.contains(&Effect::PersistQueue));
        assert!(effects.contains(&Effect::Exit));
    }

    #[test]
    fn play_track_appends_and_selects() {
        let mut state = AppState::new();
        let effects = reduce(&mut state, Action::PlayTrack(track("a")));
        assert_eq!(state.queue.tracks.len(), 1);
        assert_eq!(
            state.current_track.as_ref().map(|t| t.id.as_str()),
            Some("a")
        );
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
        );
    }

    #[test]
    fn move_selected_reorders_and_follows() {
        let mut state = AppState::new();
        state.view = View::Queue;
        state.queue.push(track("a"));
        state.queue.push(track("b"));
        state.queue.push(track("c"));
        state.queue.position = Some(0);
        state.selected_index = 0;
        let effects = reduce(&mut state, Action::MoveSelectedInQueue(1));
        assert_eq!(state.queue.order, vec![1, 0, 2]);
        assert_eq!(state.selected_index, 1);
        assert_eq!(state.queue.position, Some(1));
        assert!(effects.contains(&Effect::PersistQueue));
        // Moving past the end is a no-op.
        state.selected_index = 2;
        let effects = reduce(&mut state, Action::MoveSelectedInQueue(1));
        assert!(effects.is_empty());
    }

    #[test]
    fn eof_advances_queue() {
        let mut state = AppState::new();
        state.queue.push(track("a"));
        state.queue.push(track("b"));
        state.queue.position = Some(0);
        reduce(
            &mut state,
            Action::PlaybackEvent(PlaybackEvent::EndFile {
                reason: "eof".to_string(),
            }),
        );
        assert_eq!(state.queue.position, Some(1));
    }
}
