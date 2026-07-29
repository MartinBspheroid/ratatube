use crate::app::state::{AppState, HistoryViewMode, View};
use crate::app::track_context::{open_track_context, resolve_track_context};
use crate::history::HistoryService;
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::{history, track};

struct EmptyFilterCase {
    name: &'static str,
    state: AppState,
    history: Option<HistoryService>,
}

#[test]
fn filtered_track_views_do_not_resolve_when_no_rows_are_visible() {
    let mut queue = AppState::new();
    queue.ui.view = View::Queue;
    queue.domain.queue.push(track("queue", "Queue track"));

    let mut playlist = Playlist::new("Filtered playlist");
    playlist
        .tracks
        .push(PlaylistTrack::from(&track("playlist", "Playlist track")));
    let mut playlist_detail = AppState::new();
    playlist_detail.ui.view = View::PlaylistDetail;
    playlist_detail.domain.playlists.push(playlist);
    playlist_detail.ui.selected_playlist = Some(0);

    let mut history_recent = AppState::new();
    history_recent.ui.view = View::History;
    history_recent.ui.history_view_mode = HistoryViewMode::Recent;

    let mut history_top = AppState::new();
    history_top.ui.view = View::History;
    history_top.ui.history_view_mode = HistoryViewMode::Top;

    for case in [
        EmptyFilterCase {
            name: "queue",
            state: queue,
            history: None,
        },
        EmptyFilterCase {
            name: "playlist detail",
            state: playlist_detail,
            history: None,
        },
        EmptyFilterCase {
            name: "history recent",
            state: history_recent,
            history: Some(history(&[track("recent", "Recent history track")])),
        },
        EmptyFilterCase {
            name: "history top",
            state: history_top,
            history: Some(history(&[track("top", "Top history track")])),
        },
    ] {
        let mut state = case.state;
        state.ui.visible_indices = Some(Vec::new());

        assert!(
            resolve_track_context(&state, case.history.as_ref()).is_none(),
            "{} should not fall back to an underlying row",
            case.name
        );
    }
}

#[test]
fn resolver_does_not_fallback_when_selected_visible_row_is_absent() {
    let mut state = AppState::new();
    state.ui.view = View::Queue;
    state.domain.queue.push(track("queue", "Queue track"));
    state
        .domain
        .queue
        .push(track("hidden", "Hidden queue track"));
    state.ui.selected_index = 1;
    state.ui.visible_indices = Some(vec![0]);

    assert!(resolve_track_context(&state, None).is_none());
}

#[test]
fn opening_a_zero_match_filter_reports_no_track_and_keeps_modal_absent() {
    let mut state = AppState::new();
    state.ui.view = View::Queue;
    state.domain.queue.push(track("queue", "Queue track"));
    state.ui.visible_indices = Some(Vec::new());

    open_track_context(&mut state, None);

    assert!(state.ui.track_context_menu.is_none());
    let notification = state.ui.notification.expect("notification");
    assert_eq!(notification.message, "No track selected");
    assert!(notification.is_error);
}
