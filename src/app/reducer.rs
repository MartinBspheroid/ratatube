//! Pure state transitions: `reduce(state, action) -> effects`.
//!
//! The UI never invokes subprocesses directly; reducers return [`Effect`]s
//! that the app layer executes (PRD section 13).

use crate::app::action::Action;
use crate::app::state::{
    AppState, DetailsStatus, Focus, Notification, OperationStatus, PlayingPane, View,
};
use crate::media::search::SearchState;
use crate::playback::PlaybackEvent;
use crate::queue::PreviousOutcome;

/// Side effects the app layer must perform after a state update.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    RunSearch { query: String, generation: u64 },
    RunExactVideo { url: String, generation: u64 },
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
    PersistSession,
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
            state.search_detail_open = false;
            state.reset_list();
        }
        Action::OpenHelp => {
            if state.view != View::Help {
                state.help_return_view = state.view;
            }
            state.view = View::Help;
            state.help_scroll = 0;
            state.focus = Focus::Content;
        }
        Action::CloseHelp => {
            state.view = state.help_return_view;
            state.help_scroll = 0;
        }
        Action::ScrollHelp(delta) => {
            if state.view == View::Help {
                state.help_scroll = state.help_scroll.saturating_add_signed(delta as i16);
            }
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
        Action::ToggleSearchDetail => {
            if state.view == View::Search
                && crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                    == crate::ui::layout::Breakpoint::Narrow
            {
                state.search_detail_open = !state.search_detail_open;
            }
        }
        // Resolved by the app layer because it needs the selected track and
        // an operating-system process boundary.
        Action::OpenInBrowser => {}
        Action::ClearActivity => {
            state.activity.clear();
            return vec![Effect::PersistSession];
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
        Action::SubmitExactVideo(url) => {
            state.search_generation += 1;
            let generation = state.search_generation;
            state.search = SearchState::Searching {
                query: url.clone(),
                generation,
            };
            state.focus = Focus::Content;
            return vec![Effect::RunExactVideo { url, generation }];
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
                deadline: std::time::Instant::now()
                    + std::time::Duration::from_secs(u64::from(m) * 60),
                minutes: m,
            });
            match minutes {
                Some(m) => state.notify(&format!("Sleep timer: {m} min"), false),
                None => state.notify("Sleep timer off", false),
            }
        }
        Action::ToggleRadio => {
            state.radio = !state.radio;
            if !state.radio {
                state.radio_operation = None;
            }
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
            let started = event == PlaybackEvent::Started;
            if started && let Some(track) = &state.current_track {
                state
                    .activity
                    .push(crate::history::activity::ActivityEvent::new(
                        crate::history::activity::ActivityKind::Played,
                        track.title.clone(),
                        track.artist.clone(),
                    ));
            }
            let mut effects = reduce_playback_event(state, event);
            if started {
                effects.push(Effect::PersistSession);
            }
            return effects;
        }

        // --- Queue --------------------------------------------------------
        Action::AddToQueue(track) => {
            state
                .activity
                .push(crate::history::activity::ActivityEvent::new(
                    crate::history::activity::ActivityKind::Queued,
                    track.title.clone(),
                    track.artist.clone(),
                ));
            state.queue.push(track);
            state.notify("Added to queue", false);
            return vec![Effect::PersistQueue, Effect::PersistSession];
        }
        Action::AddNext(track) => {
            state
                .activity
                .push(crate::history::activity::ActivityEvent::new(
                    crate::history::activity::ActivityKind::Queued,
                    track.title.clone(),
                    "Play next",
                ));
            state.queue.push_next(track);
            state.notify("Will play next", false);
            return vec![Effect::PersistQueue, Effect::PersistSession];
        }
        Action::RemoveSelectedFromQueue => {
            if state.view == View::Queue {
                let real = state.resolve_index(state.selected_index);
                if let Some(track) = state.queue.remove_at(real) {
                    state.removed_queue_item = Some((real, track));
                    state.notify("Removed from queue — u to undo", false);
                }
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
        Action::UndoQueueRemoval => {
            if let Some((position, track)) = state.removed_queue_item.take() {
                state.queue.insert_at(position, track);
                state.selected_index = position;
                state.notify("Queue removal undone", false);
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
            state.confirm = Some(crate::app::state::ConfirmState {
                message: "Clear the entire queue? (y/n)".to_string(),
                action: Box::new(Action::ClearQueueConfirmed),
            });
        }
        Action::ClearQueueConfirmed => {
            state.queue.clear();
            state.removed_queue_item = None;
            state.selected_index = 0;
            return vec![Effect::PersistQueue];
        }
        Action::PlayTrack(track) => {
            state.queue.push(track);
            let pos = state.queue.order.len() - 1;
            state.queue.position = Some(pos);
            return vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: pos,
                },
                Effect::PersistQueue,
            ];
        }
        // Resolved by the app layer through the existing pending-session flow.
        Action::ResumeTrack { .. } => {}
        Action::PlaySelected => {
            if matches!(state.view, View::Queue)
                || (state.view == View::NowPlaying
                    && state.playing_pane == PlayingPane::Queue
                    && crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                        == crate::ui::layout::Breakpoint::UltraWide)
            {
                let position = state.resolve_index(state.selected_index);
                if position < state.queue.order.len() {
                    state.queue.position = Some(position);
                    return vec![
                        Effect::ResolveAndPlay {
                            track_index_in_queue: position,
                        },
                        Effect::PersistQueue,
                    ];
                }
                return Vec::new();
            }
            if let Some(track) = selected_track(state) {
                return reduce(state, Action::PlayTrack(track));
            }
        }
        Action::NextTrack => {
            if state.queue.advance().is_some() {
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
        Action::PlaybackResolveStarted { operation_id, .. } => {
            state.playback_resolution = OperationStatus::Loading { operation_id };
        }
        Action::PlaybackResolved {
            operation_id,
            queue_position,
            track_id,
            ..
        } => {
            if !matches!(
                state.playback_resolution,
                OperationStatus::Loading { operation_id: active } if active == operation_id
            ) {
                return Vec::new();
            }
            let track = state
                .queue
                .order
                .get(queue_position)
                .and_then(|index| state.queue.tracks.get(*index))
                .filter(|track| track.id == track_id)
                .cloned();
            if let Some(track) = track {
                state.current_track = Some(track);
                state.current_details = None;
                state.thumbnail = None;
                state.now_playing_scroll = 0;
                state.playback_resolution = OperationStatus::Idle;
            }
        }
        Action::PlaybackResolveFailed {
            operation_id,
            message,
            ..
        } => {
            if !matches!(
                state.playback_resolution,
                OperationStatus::Loading { operation_id: active } if active == operation_id
            ) {
                return Vec::new();
            }
            state.playback_resolution = OperationStatus::Failed {
                message: message.clone(),
            };
            state.notify(&format!("Playback unavailable: {message}"), true);
        }
        Action::DetailsStarted {
            operation_id,
            track_id,
        } => {
            state.details_status = DetailsStatus::Loading {
                operation_id,
                track_id,
            };
        }
        Action::DetailsLoaded {
            operation_id,
            track_id,
            details,
        } => {
            // Only apply details that still belong to the current track.
            if matches!(
                state.details_status,
                DetailsStatus::Loading {
                    operation_id: active,
                    track_id: ref active_track_id,
                } if active == operation_id && active_track_id == &track_id
            ) && state.current_track.as_ref().map(|t| t.id.as_str()) == Some(track_id.as_str())
            {
                state.current_details = Some(*details);
                state.details_status = DetailsStatus::Idle;
            }
        }
        Action::DetailsFailed {
            operation_id,
            track_id,
            message,
        } => {
            if matches!(
                state.details_status,
                DetailsStatus::Loading {
                    operation_id: active,
                    track_id: ref active_track_id,
                } if active == operation_id && active_track_id == &track_id
            ) && state.current_track.as_ref().map(|t| t.id.as_str()) == Some(track_id.as_str())
            {
                state.details_status = DetailsStatus::Failed { track_id, message };
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
        Action::CyclePlayingPane => {
            if state.view == View::NowPlaying
                && crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                    == crate::ui::layout::Breakpoint::UltraWide
            {
                state.playing_pane = match state.playing_pane {
                    PlayingPane::Info => PlayingPane::Queue,
                    PlayingPane::Queue => PlayingPane::Info,
                };
                if state.playing_pane == PlayingPane::Queue {
                    state.selected_index = state.queue.position.unwrap_or(0);
                }
                state.reset_list();
            }
        }
        Action::QueueExhausted => {
            state.current_track = None;
            state.notify("Queue finished", false);
        }
        Action::MixLoaded { title, tracks, .. } => {
            if tracks.is_empty() {
                state.notify("Mix came back empty", true);
                return Vec::new();
            }
            state.queue.load_tracks(tracks);
            state.queue.position = Some(0);
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
        Action::RadioRefillStarted { operation_id } => {
            state.radio_operation = Some(operation_id);
        }
        Action::RadioTracksLoaded {
            operation_id,
            tracks,
        } => {
            if !state.radio || state.radio_operation != Some(operation_id) {
                return Vec::new();
            }
            state.radio_operation = None;
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
        Action::ClearHistory => {
            state.confirm = Some(crate::app::state::ConfirmState {
                message: "Clear all playback history? (y/n)".to_string(),
                action: Box::new(Action::ClearHistoryConfirmed),
            });
        }

        // --- Modal UI ----------------------------------------------------
        Action::OpenPrompt(purpose) => {
            state.prompt = Some(crate::app::state::PromptState {
                purpose,
                buffer: String::new(),
            });
        }
        Action::PromptInput(c) => {
            if let Some(prompt) = &mut state.prompt
                && prompt.buffer.len() < 1_048_576
            {
                prompt.buffer.push(c);
            }
        }
        Action::PromptPaste(text) => {
            if let Some(prompt) = &mut state.prompt {
                let remaining = 1_048_576usize.saturating_sub(prompt.buffer.len());
                prompt.buffer.extend(text.chars().take(remaining));
            }
        }
        Action::PromptBackspace => {
            if let Some(prompt) = &mut state.prompt {
                prompt.buffer.pop();
            }
        }
        Action::PromptCancel => state.prompt = None,
        Action::OpenPlaylistEditor => {
            state.playlist_editor = state
                .selected_playlist
                .and_then(|index| state.playlists.get(index))
                .map(|playlist| crate::app::state::PlaylistEditorState {
                    name: playlist.name.clone(),
                    description: playlist.description.clone(),
                    field: crate::app::state::PlaylistEditorField::Name,
                });
        }
        Action::PlaylistEditorInput(character) => {
            if let Some(editor) = &mut state.playlist_editor {
                match editor.field {
                    crate::app::state::PlaylistEditorField::Name => editor.name.push(character),
                    crate::app::state::PlaylistEditorField::Description => {
                        editor.description.push(character);
                    }
                }
            }
        }
        Action::PlaylistEditorBackspace => {
            if let Some(editor) = &mut state.playlist_editor {
                match editor.field {
                    crate::app::state::PlaylistEditorField::Name => {
                        editor.name.pop();
                    }
                    crate::app::state::PlaylistEditorField::Description => {
                        editor.description.pop();
                    }
                }
            }
        }
        Action::PlaylistEditorNextField => {
            if let Some(editor) = &mut state.playlist_editor {
                editor.field = match editor.field {
                    crate::app::state::PlaylistEditorField::Name => {
                        crate::app::state::PlaylistEditorField::Description
                    }
                    crate::app::state::PlaylistEditorField::Description => {
                        crate::app::state::PlaylistEditorField::Name
                    }
                };
            }
        }
        Action::PlaylistEditorCancel => state.playlist_editor = None,
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
            state.sort_playlists_by_updated();
        }
        Action::DeletePlaylist(id) => {
            state.confirm = Some(crate::app::state::ConfirmState {
                message: "Delete this playlist? (local file only, y/n)".to_string(),
                action: Box::new(Action::DeletePlaylistConfirmed(id)),
            });
        }

        // --- Import --------------------------------------------------------
        Action::StartImport(url) => {
            state.prompt = None;
            return vec![Effect::RunImport { url }];
        }
        Action::ImportStarted { operation_id, url } => {
            state.import = Some(crate::app::state::ImportState::Fetching { operation_id, url });
        }
        Action::ImportCompleted {
            operation_id,
            url,
            title,
            remote_id,
            tracks,
            rejections,
        } => {
            if !matches!(
                state.import,
                Some(crate::app::state::ImportState::Fetching {
                    operation_id: active,
                    ..
                }) if active == operation_id
            ) {
                return Vec::new();
            }
            let (playlist, summary) =
                crate::playlists::import::build_import(title, url, remote_id, tracks, rejections);
            state.import = Some(crate::app::state::ImportState::Review {
                summary,
                playlist: Box::new(playlist),
            });
        }
        Action::ImportFailed {
            operation_id,
            url,
            message,
        } => {
            if !matches!(
                state.import,
                Some(crate::app::state::ImportState::Fetching {
                    operation_id: active,
                    ..
                }) if active == operation_id
            ) {
                return Vec::new();
            }
            state.import = Some(crate::app::state::ImportState::Failed {
                url,
                message: message.clone(),
            });
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
        View::NowPlaying
            if state.playing_pane == PlayingPane::Queue
                && crate::ui::layout::Breakpoint::from_width(state.screen_area.width)
                    == crate::ui::layout::Breakpoint::UltraWide =>
        {
            state
                .queue
                .order
                .get(index)
                .map(|&i| state.queue.tracks[i].clone())
        }
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
        let notification = Notification::new_at(message, is_error, std::time::Instant::now());
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
    use crate::app::operations::{OperationKind, OperationRegistry};
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
    fn help_returns_to_the_view_that_opened_it_and_scrolls() {
        let mut state = AppState::new();
        state.view = View::Queue;
        reduce(&mut state, Action::OpenHelp);
        assert_eq!(state.view, View::Help);
        assert_eq!(state.help_return_view, View::Queue);
        reduce(&mut state, Action::ScrollHelp(7));
        assert_eq!(state.help_scroll, 7);
        reduce(&mut state, Action::CloseHelp);
        assert_eq!(state.view, View::Queue);
        assert_eq!(state.help_scroll, 0);
    }

    #[test]
    fn queue_clear_requires_confirmation() {
        let mut state = AppState::new();
        state.queue.push(track("kept"));
        assert!(reduce(&mut state, Action::ClearQueue).is_empty());
        assert_eq!(state.queue.tracks.len(), 1);
        assert!(state.confirm.is_some());
        let effects = reduce(&mut state, Action::ClearQueueConfirmed);
        assert!(state.queue.tracks.is_empty());
        assert!(effects.contains(&Effect::PersistQueue));
    }

    #[test]
    fn removed_queue_item_can_be_undone() {
        let mut state = AppState::new();
        state.view = View::Queue;
        state.queue.push(track("a"));
        state.queue.push(track("b"));
        reduce(&mut state, Action::RemoveSelectedFromQueue);
        assert_eq!(state.queue.tracks[0].id, "b");
        reduce(&mut state, Action::UndoQueueRemoval);
        let restored_ids: Vec<_> = state
            .queue
            .order
            .iter()
            .map(|&index| state.queue.tracks[index].id.as_str())
            .collect();
        assert_eq!(restored_ids, ["a", "b"]);
    }

    #[test]
    fn notification_expiry_uses_elapsed_time_not_spinner_phase() {
        let now = std::time::Instant::now();
        let info = Notification::new_at("saved", false, now);
        let error = Notification::new_at("failed", true, now);
        assert!(!info.is_expired_at(now + std::time::Duration::from_secs(3)));
        assert!(info.is_expired_at(now + std::time::Duration::from_secs(5)));
        assert!(!error.is_expired_at(now + std::time::Duration::from_secs(5)));
        assert!(error.is_expired_at(now + std::time::Duration::from_secs(9)));
    }

    #[test]
    fn exact_video_uses_metadata_fetch_effect_instead_of_search() {
        let mut state = AppState::new();
        let url = "https://www.youtube.com/watch?v=exact".to_string();
        let effects = reduce(&mut state, Action::SubmitExactVideo(url.clone()));

        assert!(matches!(
            effects.as_slice(),
            [Effect::RunExactVideo { url: effect_url, .. }] if effect_url == &url
        ));
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::RunSearch { .. }))
        );
    }

    #[test]
    fn superseded_import_completion_is_discarded() {
        let mut state = AppState::new();
        let mut operations = OperationRegistry::default();
        let stale = operations.start(OperationKind::Import);
        let current = operations.start(OperationKind::Import);
        reduce(
            &mut state,
            Action::ImportStarted {
                operation_id: current.id(),
                url: "https://example.invalid/current".to_string(),
            },
        );
        reduce(
            &mut state,
            Action::ImportCompleted {
                operation_id: stale.id(),
                url: "https://example.invalid/stale".to_string(),
                title: "stale".to_string(),
                remote_id: None,
                tracks: vec![track("stale")],
                rejections: crate::media::yt_dlp::ImportRejections::default(),
            },
        );

        assert!(matches!(
            state.import,
            Some(crate::app::state::ImportState::Fetching {
                operation_id,
                ..
            }) if operation_id == current.id()
        ));
    }

    #[test]
    fn current_import_failure_has_a_terminal_error_state() {
        let mut state = AppState::new();
        let mut operations = OperationRegistry::default();
        let current = operations.start(OperationKind::Import);
        reduce(
            &mut state,
            Action::ImportStarted {
                operation_id: current.id(),
                url: "https://example.invalid/current".to_string(),
            },
        );
        reduce(
            &mut state,
            Action::ImportFailed {
                operation_id: current.id(),
                url: "https://example.invalid/current".to_string(),
                message: "offline".to_string(),
            },
        );

        assert!(matches!(
            state.import,
            Some(crate::app::state::ImportState::Failed { ref message, .. }) if message == "offline"
        ));
    }

    #[test]
    fn prompt_paste_keeps_multiline_json_as_one_input() {
        let mut state = AppState::new();
        reduce(
            &mut state,
            Action::OpenPrompt(crate::app::state::PromptPurpose::ImportPlaylistJson),
        );

        reduce(
            &mut state,
            Action::PromptPaste("{\n  \"version\": 1\n}".to_string()),
        );

        assert_eq!(
            state.prompt.as_ref().map(|prompt| prompt.buffer.as_str()),
            Some("{\n  \"version\": 1\n}")
        );
    }

    #[test]
    fn playlist_editor_copies_metadata_and_switches_fields() {
        let mut state = AppState::new();
        let mut playlist = crate::playlists::Playlist::new("Original");
        playlist.description = "Existing description".to_string();
        state.playlists.push(playlist);
        state.selected_playlist = Some(0);
        state.view = View::PlaylistDetail;

        reduce(&mut state, Action::OpenPlaylistEditor);
        assert_eq!(
            state
                .playlist_editor
                .as_ref()
                .map(|editor| editor.name.as_str()),
            Some("Original")
        );

        reduce(&mut state, Action::PlaylistEditorNextField);
        reduce(&mut state, Action::PlaylistEditorInput('!'));

        let editor = state.playlist_editor.as_ref().expect("editor open");
        assert_eq!(
            editor.field,
            crate::app::state::PlaylistEditorField::Description
        );
        assert_eq!(editor.description, "Existing description!");
    }

    #[test]
    fn details_failure_replaces_loading_for_current_track() {
        let mut state = AppState::new();
        state.current_track = Some(track("current"));
        let mut operations = OperationRegistry::default();
        let ticket = operations.start(OperationKind::Details);
        reduce(
            &mut state,
            Action::DetailsStarted {
                operation_id: ticket.id(),
                track_id: "current".to_string(),
            },
        );
        reduce(
            &mut state,
            Action::DetailsFailed {
                operation_id: ticket.id(),
                track_id: "current".to_string(),
                message: "offline".to_string(),
            },
        );

        assert!(matches!(
            state.details_status,
            DetailsStatus::Failed { ref message, .. } if message == "offline"
        ));
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
    fn play_track_waits_for_resolution_before_replacing_current_track() {
        let mut state = AppState::new();
        let effects = reduce(&mut state, Action::PlayTrack(track("a")));
        assert_eq!(state.queue.tracks.len(), 1);
        assert!(state.current_track.is_none());
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
        );

        let mut operations = OperationRegistry::default();
        let ticket = operations.start(OperationKind::Playback);
        reduce(
            &mut state,
            Action::PlaybackResolveStarted {
                operation_id: ticket.id(),
                queue_position: 0,
                track_id: "a".to_string(),
            },
        );
        reduce(
            &mut state,
            Action::PlaybackResolved {
                operation_id: ticket.id(),
                queue_position: 0,
                track_id: "a".to_string(),
                url: "https://stream.invalid/a".to_string(),
            },
        );
        assert_eq!(
            state.current_track.as_ref().map(|t| t.id.as_str()),
            Some("a")
        );
    }

    #[test]
    fn superseded_playback_completion_cannot_replace_current_track() {
        let mut state = AppState::new();
        state.queue.push(track("requested"));
        state.queue.position = Some(0);
        let mut operations = OperationRegistry::default();
        let stale = operations.start(OperationKind::Playback);
        let current = operations.start(OperationKind::Playback);
        reduce(
            &mut state,
            Action::PlaybackResolveStarted {
                operation_id: current.id(),
                queue_position: 0,
                track_id: "requested".to_string(),
            },
        );
        reduce(
            &mut state,
            Action::PlaybackResolved {
                operation_id: stale.id(),
                queue_position: 0,
                track_id: "requested".to_string(),
                url: "https://stream.invalid/stale".to_string(),
            },
        );

        assert!(state.current_track.is_none());
        assert!(matches!(
            state.playback_resolution,
            OperationStatus::Loading { operation_id } if operation_id == current.id()
        ));
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
    fn ultra_wide_playing_queue_focus_selects_without_duplicating() {
        let mut state = AppState::new();
        state.view = View::NowPlaying;
        state.screen_area = ratatui::layout::Rect::new(0, 0, 180, 48);
        state.queue.push(track("a"));
        state.queue.push(track("b"));
        state.queue.position = Some(0);

        assert!(reduce(&mut state, Action::CyclePlayingPane).is_empty());
        assert_eq!(state.playing_pane, PlayingPane::Queue);
        assert!(reduce(&mut state, Action::SelectNext).is_empty());
        assert_eq!(state.selected_index, 1);
        let effects = reduce(&mut state, Action::PlaySelected);

        assert_eq!(state.queue.tracks.len(), 2, "selection must not duplicate");
        assert_eq!(state.queue.position, Some(1));
        assert_eq!(
            effects,
            vec![
                Effect::ResolveAndPlay {
                    track_index_in_queue: 1
                },
                Effect::PersistQueue
            ]
        );
    }

    #[test]
    fn queue_and_playback_actions_emit_only_truthful_activity_kinds() {
        use crate::history::activity::ActivityKind;

        let mut state = AppState::new();
        let effects = reduce(&mut state, Action::AddToQueue(track("queued")));
        assert!(effects.contains(&Effect::PersistSession));
        assert_eq!(
            state.activity.entries().front().map(|event| event.kind),
            Some(ActivityKind::Queued)
        );

        state.current_track = Some(track("played"));
        let effects = reduce(&mut state, Action::PlaybackEvent(PlaybackEvent::Started));
        assert!(effects.contains(&Effect::PersistSession));
        assert_eq!(
            state.activity.entries().front().map(|event| event.kind),
            Some(ActivityKind::Played)
        );

        assert_eq!(
            reduce(&mut state, Action::ClearActivity),
            vec![Effect::PersistSession]
        );
        assert!(state.activity.is_empty());
    }

    #[test]
    fn chapter_jumps_seek_to_starts() {
        let mut state = AppState::new();
        state.current_track = Some(track("mix"));
        state.current_details = Some(crate::media::TrackDetails {
            chapters: vec![
                crate::media::Chapter {
                    title: "a".into(),
                    start_seconds: 0.0,
                },
                crate::media::Chapter {
                    title: "b".into(),
                    start_seconds: 100.0,
                },
                crate::media::Chapter {
                    title: "c".into(),
                    start_seconds: 200.0,
                },
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
        let mut operations = OperationRegistry::default();
        let ticket = operations.start(OperationKind::Radio);
        state.radio_operation = Some(ticket.id());
        state.queue.push(track("known"));
        // Queue already exhausted: nothing playing, no position.
        state.queue.position = None;
        let effects = reduce(
            &mut state,
            Action::RadioTracksLoaded {
                operation_id: ticket.id(),
                tracks: vec![track("known"), track("fresh1"), track("fresh2")],
            },
        );
        assert_eq!(state.queue.tracks.len(), 3, "known track deduplicated");
        assert_eq!(state.queue.position, Some(1), "starts on first fresh track");
        assert!(state.current_track.is_none());
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
        );
    }

    #[test]
    fn disabled_radio_discards_late_refill() {
        let mut state = AppState::new();
        state.radio = true;
        let mut operations = OperationRegistry::default();
        let ticket = operations.start(OperationKind::Radio);
        reduce(
            &mut state,
            Action::RadioRefillStarted {
                operation_id: ticket.id(),
            },
        );
        reduce(&mut state, Action::ToggleRadio);
        reduce(
            &mut state,
            Action::RadioTracksLoaded {
                operation_id: ticket.id(),
                tracks: vec![track("late")],
            },
        );

        assert!(state.queue.tracks.is_empty());
    }

    #[test]
    fn mix_loaded_replaces_queue_and_enables_radio() {
        let mut state = AppState::new();
        let mut operations = OperationRegistry::default();
        let ticket = operations.start(OperationKind::Mix);
        state.queue.push(track("old"));
        let effects = reduce(
            &mut state,
            Action::MixLoaded {
                operation_id: ticket.id(),
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
