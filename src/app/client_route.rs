//! Client-mode action routing (task 4b blueprint): every action either
//! stays a local UI transition, translates into a daemon command with its
//! selected-context resolved against the mirror, or is explicitly deferred.
//!
//! The match is deliberately wildcard-free so adding an action variant
//! forces a routing decision at compile time.

use crate::app::action::{
    Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction,
};
use crate::app::state::{AppState, PlayingPane, View};
use crate::history::HistoryService;
use crate::protocol::Command;

/// Where a client-mode action goes.
#[derive(Debug)]
pub(crate) enum Route {
    /// Apply through the local reducer/service path (UI-only work).
    Local,
    /// End the client loop without persisting anything.
    Quit,
    /// Send to the daemon; the mirror updates via broadcast events.
    Send(Command),
    /// Client-owned search: mark the mirror as searching, then send.
    Search { query: String, exact: bool },
    /// Prompt submission is purpose-dependent; the runtime translates it.
    PromptSubmit,
    /// Picker submission resolves its candidate list in the runtime.
    PickerSubmit,
    /// Playlist-editor submission resolves the selected id in the runtime.
    EditorSubmit,
    /// Open the channel browser: the runtime records the return snapshot.
    OpenChannel(crate::media::Track),
    /// Leave the channel browser: the runtime restores its snapshot.
    ChannelBack,
    /// Blocked by a state guard; the runtime shows the message.
    Deferred(&'static str),
    /// Daemon-internal completion or no-op in client mode.
    Ignore,
}

/// Classify one action for the client runtime.
pub(crate) fn route(action: &Action, state: &AppState, history: Option<&HistoryService>) -> Route {
    match action {
        Action::Navigation(action) => route_navigation(action),
        Action::Playback(action) => route_playback(action, state, history),
        Action::Queue(action) => route_queue(action, state, history),
        Action::Playlists(action) => route_playlists(action, state),
        Action::History(action) => route_history(action, state, history),
    }
}

fn route_navigation(action: &NavigationAction) -> Route {
    match action {
        NavigationAction::Quit => Route::Quit,
        NavigationAction::SubmitSearch(query) => Route::Search {
            query: query.clone(),
            exact: false,
        },
        NavigationAction::SubmitExactVideo(url) => Route::Search {
            query: url.clone(),
            exact: true,
        },
        NavigationAction::VisitChannel(track) => Route::OpenChannel(track.clone()),
        NavigationAction::LoadMoreChannel | NavigationAction::RetryChannel => {
            Route::Send(Command::ChannelPage)
        }
        NavigationAction::BackFromChannel => Route::ChannelBack,
        NavigationAction::ChannelResolved { .. }
        | NavigationAction::ChannelPageLoaded { .. }
        | NavigationAction::SearchCompleted { .. }
        | NavigationAction::SearchFailed { .. } => Route::Ignore,
        NavigationAction::Navigate(_)
        | NavigationAction::OpenHelp
        | NavigationAction::CloseHelp
        | NavigationAction::ScrollHelp(_)
        | NavigationAction::NextView
        | NavigationAction::PreviousView
        | NavigationAction::CycleHomeSection(_)
        | NavigationAction::SearchInput(_)
        | NavigationAction::SearchBackspace
        | NavigationAction::ClearSearch
        | NavigationAction::ToggleSearchDetail
        | NavigationAction::OpenInBrowser
        | NavigationAction::ExternalCommandCompleted { .. }
        | NavigationAction::OpenTrackContext
        | NavigationAction::CloseTrackContext
        | NavigationAction::MoveTrackContext(_)
        | NavigationAction::SubmitTrackContext
        | NavigationAction::ShowTrackDetails(_)
        | NavigationAction::CloseTrackDetails
        | NavigationAction::SelectNext
        | NavigationAction::SelectPrevious => Route::Local,
    }
}

fn route_playback(
    action: &PlaybackAction,
    state: &AppState,
    history: Option<&HistoryService>,
) -> Route {
    match action {
        PlaybackAction::PlayPause => Route::Send(Command::PlayPause),
        PlaybackAction::Stop => Route::Send(Command::Stop),
        PlaybackAction::NextTrack => Route::Send(Command::Next),
        PlaybackAction::PreviousTrack => Route::Send(Command::Previous),
        PlaybackAction::SeekForward => Route::Send(Command::Seek { seconds: 5 }),
        PlaybackAction::SeekBackward => Route::Send(Command::Seek { seconds: -5 }),
        PlaybackAction::SeekForwardLarge => Route::Send(Command::Seek { seconds: 30 }),
        PlaybackAction::SeekBackwardLarge => Route::Send(Command::Seek { seconds: -30 }),
        PlaybackAction::SeekBy(seconds) => Route::Send(Command::Seek { seconds: *seconds }),
        PlaybackAction::SeekToSeconds(seconds) => {
            Route::Send(Command::SeekAbsolute { seconds: *seconds })
        }
        PlaybackAction::SeekToFraction(fraction) => match state.domain.playback.duration_seconds {
            Some(duration) => Route::Send(Command::SeekAbsolute {
                seconds: duration * fraction.clamp(0.0, 1.0),
            }),
            None => Route::Ignore,
        },
        PlaybackAction::VolumeUp => Route::Send(Command::Volume { delta: 2 }),
        PlaybackAction::VolumeDown => Route::Send(Command::Volume { delta: -2 }),
        PlaybackAction::VolumeBy(delta) => Route::Send(Command::Volume { delta: *delta }),
        PlaybackAction::ToggleMute => Route::Send(Command::ToggleMute),
        PlaybackAction::ToggleShuffle => Route::Send(Command::ToggleShuffle),
        PlaybackAction::CycleRepeat => Route::Send(Command::CycleRepeat),
        PlaybackAction::SpeedUp => Route::Send(Command::SpeedUp),
        PlaybackAction::SpeedDown => Route::Send(Command::SpeedDown),
        PlaybackAction::SpeedReset => Route::Send(Command::SpeedReset),
        PlaybackAction::CycleSleepTimer => Route::Send(Command::CycleSleepTimer),
        PlaybackAction::ToggleRadio => Route::Send(Command::ToggleRadio),
        PlaybackAction::PlayTrack(track) => Route::Send(Command::PlayTrack {
            track: track.clone(),
        }),
        PlaybackAction::PlayQueuePosition(position) => Route::Send(Command::PlayQueuePosition {
            position: *position,
        }),
        PlaybackAction::ResumeTrack {
            track,
            position_seconds,
        } => Route::Send(Command::Resume {
            track: track.clone(),
            position_seconds: *position_seconds,
        }),
        PlaybackAction::PlaySelected => route_play_selected(state, history),
        PlaybackAction::NextChapter => next_chapter_target(state)
            .map(|seconds| Route::Send(Command::SeekAbsolute { seconds }))
            .unwrap_or(Route::Ignore),
        PlaybackAction::PreviousChapter => previous_chapter_target(state)
            .map(|seconds| Route::Send(Command::SeekAbsolute { seconds }))
            .unwrap_or(Route::Ignore),
        // Presentation-only transitions and local thumbnail completions.
        PlaybackAction::ScrollNowPlaying(_)
        | PlaybackAction::ToggleNowPlayingPane
        | PlaybackAction::CyclePlayingPane
        | PlaybackAction::ThumbnailLoaded { .. }
        | PlaybackAction::SearchThumbnailLoaded { .. } => Route::Local,
        // Daemon-internal completions and supervision results never arise
        // from client input; drop them defensively.
        PlaybackAction::SessionStreamResolved { .. }
        | PlaybackAction::SessionResolveFailed { .. }
        | PlaybackAction::PlaybackResolveStarted { .. }
        | PlaybackAction::PlaybackResolved { .. }
        | PlaybackAction::PlaybackResolveFailed { .. }
        | PlaybackAction::PlaybackEvent(_)
        | PlaybackAction::DetailsStarted { .. }
        | PlaybackAction::DetailsLoaded { .. }
        | PlaybackAction::DetailsFailed { .. }
        | PlaybackAction::PrefetchResolved { .. }
        | PlaybackAction::RadioRefillStarted { .. }
        | PlaybackAction::RadioTracksLoaded { .. }
        | PlaybackAction::MixLoaded { .. }
        | PlaybackAction::RepeatChanged(_) => Route::Ignore,
    }
}

fn route_play_selected(state: &AppState, history: Option<&HistoryService>) -> Route {
    let queue_pane = state.ui.view == View::Queue
        || (state.ui.view == View::NowPlaying
            && state.ui.playing_pane == PlayingPane::Queue
            && crate::ui::layout::Breakpoint::from_width(state.ui.screen_area.width)
                == crate::ui::layout::Breakpoint::UltraWide);
    if queue_pane {
        let position = state.resolve_index(state.ui.selected_index);
        if position < state.domain.queue.order.len() {
            return Route::Send(Command::PlayQueuePosition { position });
        }
        return Route::Ignore;
    }
    match crate::app::selection::resolve_selected_track(state, history) {
        Some(track) => Route::Send(Command::PlayTrack { track }),
        None => Route::Ignore,
    }
}

fn route_queue(action: &QueueAction, state: &AppState, history: Option<&HistoryService>) -> Route {
    match action {
        QueueAction::AddToQueue(track) => Route::Send(Command::QueueAdd {
            track: track.clone(),
            next: false,
        }),
        QueueAction::AddNext(track) => Route::Send(Command::QueueAdd {
            track: track.clone(),
            next: true,
        }),
        QueueAction::AddSelectedToQueue => {
            match crate::app::selection::resolve_selected_track(state, history) {
                Some(track) => Route::Send(Command::QueueAdd { track, next: false }),
                None => Route::Ignore,
            }
        }
        QueueAction::AddSelectedAsNext => {
            match crate::app::selection::resolve_selected_track(state, history) {
                Some(track) => Route::Send(Command::QueueAdd { track, next: true }),
                None => Route::Ignore,
            }
        }
        QueueAction::RemoveSelectedFromQueue => {
            if state.ui.view != View::Queue {
                return Route::Ignore;
            }
            Route::Send(Command::QueueRemove {
                order_index: state.resolve_index(state.ui.selected_index),
                expected_revision: state.domain.queue_revision,
            })
        }
        QueueAction::RemoveTrackOccurrence {
            order_index,
            expected_revision,
            ..
        } => Route::Send(Command::QueueRemove {
            order_index: *order_index,
            expected_revision: *expected_revision,
        }),
        QueueAction::UndoQueueRemoval => Route::Send(Command::QueueUndo),
        QueueAction::MoveSelectedInQueue(delta) => {
            if state.ui.visible_indices.is_some() {
                return Route::Deferred("Clear the filter (Esc) to reorder");
            }
            let len = state.domain.queue.order.len();
            let from = state.ui.selected_index;
            let to = from
                .saturating_add_signed(*delta as isize)
                .min(len.saturating_sub(1));
            if state.ui.view == View::Queue && len > 1 && from != to {
                Route::Send(Command::QueueMove { from, to })
            } else {
                Route::Ignore
            }
        }
        QueueAction::MoveTrack { from, to } => Route::Send(Command::QueueMove {
            from: *from,
            to: *to,
        }),
        QueueAction::ClearQueue => Route::Local,
        QueueAction::ClearQueueConfirmed => Route::Send(Command::QueueClear),
        QueueAction::LoadSelectedPlaylistIntoQueue => selected_playlist_load(state, false),
        QueueAction::AppendSelectedPlaylistToQueue => selected_playlist_load(state, true),
        QueueAction::QueueLoaded(_) | QueueAction::QueueExhausted => Route::Ignore,
    }
}

fn selected_playlist_load(state: &AppState, append: bool) -> Route {
    let index = state.resolve_index(state.ui.selected_index);
    match state.domain.playlists.get(index) {
        Some(playlist) => Route::Send(Command::PlaylistLoad {
            id: playlist.id.clone(),
            append,
        }),
        None => Route::Ignore,
    }
}

fn route_playlists(action: &PlaylistAction, state: &AppState) -> Route {
    match action {
        PlaylistAction::LoadPlaylistIntoQueue(id) => Route::Send(Command::PlaylistLoad {
            id: id.clone(),
            append: false,
        }),
        PlaylistAction::AppendPlaylistToQueue(id) => Route::Send(Command::PlaylistLoad {
            id: id.clone(),
            append: true,
        }),
        PlaylistAction::DeletePlaylistConfirmed(id) => {
            Route::Send(Command::PlaylistDelete { id: id.clone() })
        }
        PlaylistAction::CreatePlaylist(name) => {
            Route::Send(Command::PlaylistCreate { name: name.clone() })
        }
        PlaylistAction::SaveQueueAsPlaylist(name) => {
            Route::Send(Command::SaveQueueAsPlaylist { name: name.clone() })
        }
        PlaylistAction::RemoveTrackOccurrence {
            playlist_id,
            track_index,
            expected_revision,
            ..
        } => Route::Send(Command::PlaylistRemoveTrack {
            playlist_id: playlist_id.clone(),
            track_index: *track_index,
            expected_revision: *expected_revision,
        }),
        PlaylistAction::RemoveSelectedFromPlaylist => {
            if state.ui.view != View::PlaylistDetail {
                return Route::Ignore;
            }
            let Some(playlist) = state
                .ui
                .selected_playlist
                .and_then(|index| state.domain.playlists.get(index))
            else {
                return Route::Ignore;
            };
            Route::Send(Command::PlaylistRemoveTrack {
                playlist_id: playlist.id.clone(),
                track_index: state.resolve_index(state.ui.selected_index),
                expected_revision: state.domain.playlists_revision,
            })
        }
        PlaylistAction::PromptSubmit => Route::PromptSubmit,
        PlaylistAction::PickerSubmit => Route::PickerSubmit,
        PlaylistAction::PlaylistEditorSubmit => Route::EditorSubmit,
        PlaylistAction::RenameSelectedPlaylist(name) => {
            let index = state.resolve_index(state.ui.selected_index);
            match state.domain.playlists.get(index) {
                Some(playlist) => Route::Send(Command::PlaylistRename {
                    id: playlist.id.clone(),
                    name: name.clone(),
                }),
                None => Route::Ignore,
            }
        }
        PlaylistAction::AddTrackToPlaylist { playlist_id, track } => {
            Route::Send(Command::PlaylistAddTrack {
                playlist_id: playlist_id.clone(),
                track: track.clone(),
            })
        }
        PlaylistAction::AddTrackToNewPlaylist { name, track } => {
            Route::Send(Command::PlaylistAddTrackNew {
                name: name.clone(),
                track: track.clone(),
            })
        }
        PlaylistAction::RenamePlaylist { id, name } => Route::Send(Command::PlaylistRename {
            id: id.clone(),
            name: name.clone(),
        }),
        PlaylistAction::EditPlaylist {
            id,
            name,
            description,
        } => Route::Send(Command::PlaylistEdit {
            id: id.clone(),
            name: name.clone(),
            description: description.clone(),
        }),
        PlaylistAction::MoveTrackInPlaylist { id, from, to } => {
            Route::Send(Command::PlaylistMoveTrack {
                id: id.clone(),
                from: *from,
                to: *to,
            })
        }
        PlaylistAction::MoveSelectedInPlaylist(delta) => {
            if state.ui.view != View::PlaylistDetail {
                return Route::Ignore;
            }
            if state.ui.visible_indices.is_some() {
                return Route::Deferred("Clear the filter (Esc) to reorder");
            }
            let Some(playlist) = state
                .ui
                .selected_playlist
                .and_then(|index| state.domain.playlists.get(index))
            else {
                return Route::Ignore;
            };
            let len = playlist.tracks.len();
            let from = state.ui.selected_index;
            let to = from
                .saturating_add_signed(*delta as isize)
                .min(len.saturating_sub(1));
            if len > 1 && from < len && from != to {
                Route::Send(Command::PlaylistMoveTrack {
                    id: playlist.id.clone(),
                    from,
                    to,
                })
            } else {
                Route::Ignore
            }
        }
        PlaylistAction::StartImport(url) => Route::Send(Command::ImportStart { url: url.clone() }),
        PlaylistAction::ConfirmImport => Route::Send(Command::ImportConfirm),
        PlaylistAction::CancelImport => Route::Send(Command::ImportCancel),
        PlaylistAction::ImportPlaylistsJson(json) => {
            Route::Send(Command::ImportJson { json: json.clone() })
        }
        PlaylistAction::ImportStarted { .. }
        | PlaylistAction::ImportCompleted { .. }
        | PlaylistAction::ImportFailed { .. } => Route::Ignore,
        PlaylistAction::PlaylistSaved(_) => Route::Ignore,
        PlaylistAction::DeleteSelectedPlaylist
        | PlaylistAction::OpenPlaylistDetail
        | PlaylistAction::DeletePlaylist(_)
        | PlaylistAction::OpenPrompt(_)
        | PlaylistAction::PromptInput(_)
        | PlaylistAction::PromptPaste(_)
        | PlaylistAction::PromptBackspace
        | PlaylistAction::PromptCancel
        | PlaylistAction::OpenPlaylistEditor
        | PlaylistAction::PlaylistEditorInput(_)
        | PlaylistAction::PlaylistEditorBackspace
        | PlaylistAction::PlaylistEditorNextField
        | PlaylistAction::PlaylistEditorCancel
        | PlaylistAction::ConfirmYes
        | PlaylistAction::ConfirmNo
        | PlaylistAction::OpenPlaylistPicker
        | PlaylistAction::OpenPlaylistPickerForTrack(_)
        | PlaylistAction::PickerInput(_)
        | PlaylistAction::PickerBackspace
        | PlaylistAction::PickerNext
        | PlaylistAction::PickerPrevious
        | PlaylistAction::PickerCancel => Route::Local,
    }
}

fn route_history(
    action: &HistoryAction,
    state: &AppState,
    history: Option<&HistoryService>,
) -> Route {
    match action {
        HistoryAction::ClearHistoryConfirmed => Route::Send(Command::HistoryClear),
        HistoryAction::ClearActivity => Route::Send(Command::ActivityClear),
        HistoryAction::DeleteSelectedHistoryEntry => {
            if state.ui.view != View::History
                || state.ui.history_view_mode != crate::app::state::HistoryViewMode::Recent
            {
                return Route::Deferred("Switch to recent (g) to delete entries");
            }
            let index = state.resolve_index(state.ui.selected_index);
            match history.and_then(|history| history.entries().get(index)) {
                Some(entry) => Route::Send(Command::HistoryDelete {
                    index,
                    expected_track_id: entry.track_id.clone(),
                }),
                None => Route::Ignore,
            }
        }
        HistoryAction::DeleteHistoryEntry {
            index,
            expected_track_id,
        } => Route::Send(Command::HistoryDelete {
            index: *index,
            expected_track_id: expected_track_id.clone(),
        }),
        HistoryAction::ClearHistory
        | HistoryAction::ToggleHistoryViewMode
        | HistoryAction::Notify(_)
        | HistoryAction::DismissNotification
        | HistoryAction::ToggleNotificationLog => Route::Local,
    }
}

/// Next chapter boundary after the playhead (mirror-side computation).
fn next_chapter_target(state: &AppState) -> Option<f64> {
    let position = state.domain.playback.position_seconds;
    state
        .domain
        .chapters()
        .iter()
        .find(|c| c.start_seconds > position + 1.0)
        .map(|c| c.start_seconds)
}

/// Restart the current chapter or step back (mirror-side computation).
fn previous_chapter_target(state: &AppState) -> Option<f64> {
    let chapters = state.domain.chapters();
    let current = state.domain.current_chapter_index()?;
    let start = chapters[current].start_seconds;
    Some(
        if state.domain.playback.position_seconds > start + 3.0 || current == 0 {
            start
        } else {
            chapters[current - 1].start_seconds
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Track;

    fn track(id: &str) -> Track {
        Track::new(id, id, "channel")
    }

    #[test]
    fn quit_never_reaches_the_daemon_or_local_reduce() {
        let state = AppState::new();
        assert!(matches!(
            route(&Action::Navigation(NavigationAction::Quit), &state, None),
            Route::Quit
        ));
    }

    #[test]
    fn search_routes_as_client_owned_search() {
        let state = AppState::new();
        let routed = route(
            &Action::Navigation(NavigationAction::SubmitSearch("q".into())),
            &state,
            None,
        );
        assert!(matches!(routed, Route::Search { exact: false, .. }));
    }

    #[test]
    fn queue_removal_carries_the_mirror_revision() {
        let mut state = AppState::new();
        state.ui.view = View::Queue;
        state.domain.queue.push(track("a"));
        state.domain.queue_revision = 9;
        let routed = route(
            &Action::Queue(QueueAction::RemoveSelectedFromQueue),
            &state,
            None,
        );
        match routed {
            Route::Send(Command::QueueRemove {
                order_index: 0,
                expected_revision: 9,
            }) => {}
            other => panic!("unexpected route: {other:?}"),
        }
    }

    #[test]
    fn play_selected_in_queue_view_jumps_by_position() {
        let mut state = AppState::new();
        state.ui.view = View::Queue;
        state.domain.queue.push(track("a"));
        state.domain.queue.push(track("b"));
        state.ui.selected_index = 1;
        let routed = route(
            &Action::Playback(PlaybackAction::PlaySelected),
            &state,
            None,
        );
        assert!(matches!(
            routed,
            Route::Send(Command::PlayQueuePosition { position: 1 })
        ));
    }

    #[test]
    fn channel_actions_route_through_the_snapshot_stack() {
        let state = AppState::new();
        assert!(matches!(
            route(
                &Action::Navigation(NavigationAction::VisitChannel(track("c"))),
                &state,
                None
            ),
            Route::OpenChannel(_)
        ));
        assert!(matches!(
            route(
                &Action::Navigation(NavigationAction::BackFromChannel),
                &state,
                None
            ),
            Route::ChannelBack
        ));
        assert!(matches!(
            route(
                &Action::Navigation(NavigationAction::LoadMoreChannel),
                &state,
                None
            ),
            Route::Send(Command::ChannelPage)
        ));
    }

    #[test]
    fn picker_and_editor_submissions_resolve_in_the_runtime() {
        let state = AppState::new();
        assert!(matches!(
            route(
                &Action::Playlists(PlaylistAction::PickerSubmit),
                &state,
                None
            ),
            Route::PickerSubmit
        ));
        assert!(matches!(
            route(
                &Action::Playlists(PlaylistAction::PlaylistEditorSubmit),
                &state,
                None
            ),
            Route::EditorSubmit
        ));
    }
}
