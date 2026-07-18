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
    SetSpeed(f64),
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
            state.list_filter = None;
            state.visible_indices = None;
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
        Action::SpeedUp => return speed_step(state, 0.25),
        Action::SpeedDown => return speed_step(state, -0.25),
        Action::SpeedReset => {
            if (state.playback.speed - 1.0).abs() > f64::EPSILON {
                state.notify("Speed 1.00x", false);
                return vec![Effect::SetSpeed(1.0)];
            }
        }
        Action::CycleSleepTimer => {
            use crate::app::state::SleepTimer;
            let minutes = match state.sleep_timer.map(|t| t.minutes) {
                None => Some(15),
                Some(15) => Some(30),
                Some(30) => Some(60),
                Some(_) => None,
            };
            state.sleep_timer = minutes.map(|m| SleepTimer {
                deadline: std::time::Instant::now() + std::time::Duration::from_secs(u64::from(m) * 60),
                minutes: m,
            });
            match minutes {
                Some(m) => state.notify(&format!("Sleep timer: {m} min"), false),
                None => state.notify("Sleep timer off", false),
            }
        }
        Action::ToggleRadio => {
            state.radio = !state.radio;
            state.notify(
                if state.radio {
                    "Radio on: the queue will keep itself filled"
                } else {
                    "Radio off"
                },
                false,
            );
        }
        Action::ToggleNotificationLog => {
            state.show_notification_log = !state.show_notification_log;
        }
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
                let real = state.resolve_index(state.selected_index);
                state.queue.remove_at(real);
                // The filter indices refresh next loop; drop them now so the
                // stale mapping can't resolve a second removal wrongly.
                if let Some(indices) = &mut state.visible_indices {
                    indices.retain(|&i| i != real);
                    for i in indices.iter_mut() {
                        if *i > real {
                            *i -= 1;
                        }
                    }
                }
                state.clamp_selection();
                return vec![Effect::PersistQueue];
            }
        }
        Action::MoveSelectedInQueue(delta) => {
            // Reordering a filtered view would move hidden neighbors around
            // invisibly; require the full list.
            if state.visible_indices.is_some() {
                state.notify("Clear the filter (Esc) to reorder", false);
                return Vec::new();
            }
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
        Action::NextChapter => {
            let position = state.playback.position_seconds;
            if let Some(chapter) = state
                .chapters()
                .iter()
                .find(|c| c.start_seconds > position + 1.0)
            {
                return vec![Effect::SeekTo(chapter.start_seconds)];
            }
        }
        Action::PreviousChapter => {
            let chapters = state.chapters();
            if let Some(current) = state.current_chapter_index() {
                let start = chapters[current].start_seconds;
                // Like PreviousTrack: restart the current chapter first,
                // then step back to the one before it.
                let target = if state.playback.position_seconds > start + 3.0 || current == 0 {
                    start
                } else {
                    chapters[current - 1].start_seconds
                };
                return vec![Effect::SeekTo(target)];
            }
        }
        Action::ToggleNowPlayingPane => {
            state.now_playing_show_description = !state.now_playing_show_description;
            state.now_playing_scroll = 0;
        }
        Action::QueueExhausted => {
            state.current_track = None;
            state.notify("Queue finished", false);
        }
        Action::MixLoaded { title, tracks } => {
            if tracks.is_empty() {
                state.notify("Mix came back empty", true);
                return Vec::new();
            }
            state.queue.load_tracks(tracks);
            state.queue.position = Some(0);
            state.current_track = state.queue.current().cloned();
            state.current_details = None;
            state.thumbnail = None;
            state.now_playing_scroll = 0;
            state.radio = true;
            state.notify(&format!("Playing mix: {title}"), false);
            return vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: 0,
                },
                Effect::PersistQueue,
            ];
        }
        Action::RadioTracksLoaded { tracks } => {
            let known: std::collections::HashSet<String> =
                state.queue.tracks.iter().map(|t| t.id.clone()).collect();
            let fresh: Vec<_> = tracks
                .into_iter()
                .filter(|t| !known.contains(&t.id))
                .take(10)
                .collect();
            if fresh.is_empty() {
                return Vec::new();
            }
            let first_new = state.queue.order.len();
            let count = fresh.len();
            for track in fresh {
                state.queue.push(track);
            }
            state.notify(&format!("Radio: added {count} tracks"), false);
            // If playback had already run dry, start on the new tracks.
            if state.queue.position.is_none() || state.current_track.is_none() {
                state.queue.position = Some(first_new);
                state.current_track = state.queue.current().cloned();
                state.current_details = None;
                state.thumbnail = None;
                state.now_playing_scroll = 0;
                return vec![
                    Effect::ResolveAndPlay {
                        track_index_in_queue: first_new,
                    },
                    Effect::PersistQueue,
                ];
            }
            return vec![Effect::PersistQueue];
        }

        // --- Add-to-playlist picker ----------------------------------------
        Action::PickerInput(c) => {
            if let Some(picker) = &mut state.picker {
                picker.filter.push(c);
                picker.selected = 0;
            }
        }
        Action::PickerBackspace => {
            if let Some(picker) = &mut state.picker {
                picker.filter.pop();
                picker.selected = 0;
            }
        }
        Action::PickerNext => picker_move(state, 1),
        Action::PickerPrevious => picker_move(state, -1),
        Action::PickerCancel => state.picker = None,

        // --- History presentation ------------------------------------------
        Action::ToggleHistoryViewMode => {
            state.history_view_mode = match state.history_view_mode {
                crate::app::state::HistoryViewMode::Recent => {
                    crate::app::state::HistoryViewMode::Top
                }
                crate::app::state::HistoryViewMode::Top => {
                    crate::app::state::HistoryViewMode::Recent
                }
            };
            state.selected_index = 0;
            state.reset_list();
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
                state.selected_playlist = Some(state.resolve_index(state.selected_index));
                state.view = View::PlaylistDetail;
                state.selected_index = 0;
                state.list_filter = None;
                state.visible_indices = None;
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

/// Step the playback speed by `delta`, clamped to 0.5–2.0.
fn speed_step(state: &mut AppState, delta: f64) -> Vec<Effect> {
    let target = (state.playback.speed + delta).clamp(0.5, 2.0);
    if (target - state.playback.speed).abs() > f64::EPSILON {
        state.notify(&format!("Speed {target:.2}x"), false);
        return vec![Effect::SetSpeed(target)];
    }
    Vec::new()
}

/// Move the picker selection through its candidate list, wrapping.
fn picker_move(state: &mut AppState, delta: i32) {
    let Some(picker) = &state.picker else {
        return;
    };
    let (create_new, matching) =
        crate::app::filter::picker_candidates(&state.playlists, &picker.filter);
    let total = usize::from(create_new) + matching.len();
    if total > 0
        && let Some(picker) = &mut state.picker
    {
        picker.selected = (picker.selected as i32 + delta).rem_euclid(total as i32) as usize;
    }
}

/// Track selected in the active view, if the view lists tracks. The
/// selection index maps through the in-list filter when one is active.
fn selected_track(state: &AppState) -> Option<crate::media::Track> {
    let index = state.resolve_index(state.selected_index);
    match state.view {
        View::Search => match &state.search {
            SearchState::Results { tracks, .. } => tracks.get(index).cloned(),
            _ => None,
        },
        View::Queue => state
            .queue
            .order
            .get(index)
            .map(|&i| state.queue.tracks[i].clone()),
        View::PlaylistDetail => state
            .selected_playlist
            .and_then(|i| state.playlists.get(i))
            .and_then(|p| p.tracks.get(index))
            .map(crate::media::Track::from),
        _ => None,
    }
}

impl AppState {
    /// Record a transient notification (also kept in the `!` log).
    pub fn notify(&mut self, message: &str, is_error: bool) {
        let notification = Notification {
            message: message.to_string(),
            is_error,
        };
        self.notification_log.push_front(notification.clone());
        self.notification_log.truncate(50);
        self.notification = Some(notification);
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
            PlaybackEvent::SpeedChanged(s) => self.playback.speed = *s,
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
    fn chapter_jumps_seek_to_starts() {
        let mut state = AppState::new();
        state.current_track = Some(track("mix"));
        state.current_details = Some(crate::media::TrackDetails {
            chapters: vec![
                crate::media::Chapter { title: "a".into(), start_seconds: 0.0 },
                crate::media::Chapter { title: "b".into(), start_seconds: 100.0 },
                crate::media::Chapter { title: "c".into(), start_seconds: 200.0 },
            ],
            ..Default::default()
        });
        state.playback.position_seconds = 120.0;
        assert_eq!(
            reduce(&mut state, Action::NextChapter),
            vec![Effect::SeekTo(200.0)]
        );
        // More than 3s into a chapter: restart it first.
        assert_eq!(
            reduce(&mut state, Action::PreviousChapter),
            vec![Effect::SeekTo(100.0)]
        );
        // Near the chapter start: go to the previous one.
        state.playback.position_seconds = 101.0;
        assert_eq!(
            reduce(&mut state, Action::PreviousChapter),
            vec![Effect::SeekTo(0.0)]
        );
        // No chapters: no effects.
        state.current_details = None;
        assert!(reduce(&mut state, Action::NextChapter).is_empty());
    }

    #[test]
    fn speed_steps_clamp() {
        let mut state = AppState::new();
        assert_eq!(
            reduce(&mut state, Action::SpeedUp),
            vec![Effect::SetSpeed(1.25)]
        );
        state.playback.speed = 2.0;
        assert!(reduce(&mut state, Action::SpeedUp).is_empty());
        state.playback.speed = 0.5;
        assert!(reduce(&mut state, Action::SpeedDown).is_empty());
        state.playback.speed = 1.5;
        assert_eq!(
            reduce(&mut state, Action::SpeedReset),
            vec![Effect::SetSpeed(1.0)]
        );
    }

    #[test]
    fn sleep_timer_cycles_through_durations() {
        let mut state = AppState::new();
        reduce(&mut state, Action::CycleSleepTimer);
        assert_eq!(state.sleep_timer.map(|t| t.minutes), Some(15));
        reduce(&mut state, Action::CycleSleepTimer);
        assert_eq!(state.sleep_timer.map(|t| t.minutes), Some(30));
        reduce(&mut state, Action::CycleSleepTimer);
        assert_eq!(state.sleep_timer.map(|t| t.minutes), Some(60));
        reduce(&mut state, Action::CycleSleepTimer);
        assert!(state.sleep_timer.is_none());
    }

    #[test]
    fn radio_tracks_dedup_and_restart_playback() {
        let mut state = AppState::new();
        state.radio = true;
        state.queue.push(track("known"));
        // Queue already exhausted: nothing playing, no position.
        state.queue.position = None;
        let effects = reduce(
            &mut state,
            Action::RadioTracksLoaded {
                tracks: vec![track("known"), track("fresh1"), track("fresh2")],
            },
        );
        assert_eq!(state.queue.tracks.len(), 3, "known track deduplicated");
        assert_eq!(state.queue.position, Some(1), "starts on first fresh track");
        assert_eq!(
            state.current_track.as_ref().map(|t| t.id.as_str()),
            Some("fresh1")
        );
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
        );
    }

    #[test]
    fn mix_loaded_replaces_queue_and_enables_radio() {
        let mut state = AppState::new();
        state.queue.push(track("old"));
        let effects = reduce(
            &mut state,
            Action::MixLoaded {
                title: "My Mix".to_string(),
                tracks: vec![track("m1"), track("m2")],
            },
        );
        assert!(state.radio);
        assert_eq!(state.queue.tracks.len(), 2);
        assert_eq!(state.queue.position, Some(0));
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
        );
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
