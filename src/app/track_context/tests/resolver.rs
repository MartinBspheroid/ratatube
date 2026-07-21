use crate::app::channel::{ChannelNavigationSnapshot, ChannelState};
use crate::app::state::{AppState, Focus, HistoryViewMode, HomeSection, View};
use crate::media::search::SearchState;
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::{ResolverCase, TrackContextAction, TrackSource, history, standard_actions, track};
use crate::app::track_context::resolve_track_context;

fn resolver_cases() -> Vec<ResolverCase> {
    let mut search = AppState::new();
    search.ui.view = View::Search;
    search.domain.search = SearchState::Results {
        query: "search".to_string(),
        tracks: vec![track("search", "Search result")],
    };

    let mut queue = AppState::new();
    queue.ui.view = View::Queue;
    queue
        .domain
        .queue
        .push(track("duplicate", "First queue occurrence"));
    queue
        .domain
        .queue
        .push(track("duplicate", "Second queue occurrence"));
    queue.domain.queue.order = vec![1, 0];
    queue.ui.selected_index = 1;
    let mut queue_actions = standard_actions(false);
    queue_actions.push(TrackContextAction::RemoveFromQueue { order_index: 1 });

    let mut playlist = Playlist::new("Duplicates");
    let playlist_id = playlist.id.clone();
    playlist.tracks.push(PlaylistTrack::from(&track(
        "duplicate",
        "First playlist occurrence",
    )));
    playlist.tracks.push(PlaylistTrack::from(&track(
        "duplicate",
        "Second playlist occurrence",
    )));
    let mut playlist_detail = AppState::new();
    playlist_detail.ui.view = View::PlaylistDetail;
    playlist_detail.domain.playlists.push(playlist);
    playlist_detail.ui.selected_playlist = Some(0);
    playlist_detail.ui.selected_index = 1;
    let mut playlist_actions = standard_actions(true);
    playlist_actions.push(TrackContextAction::RemoveFromPlaylist {
        playlist_id: playlist_id.clone(),
        track_index: 1,
    });

    let recent_history = history(&[
        track("recent-old", "Older history track"),
        track("recent-new", "Recent history track"),
    ]);
    let mut recent = AppState::new();
    recent.ui.view = View::History;
    recent.ui.history_view_mode = HistoryViewMode::Recent;

    let top_history = history(&[
        track("top-other", "Other top track"),
        track("top", "Top history track"),
        track("top", "Top history track"),
    ]);
    let mut top = AppState::new();
    top.ui.view = View::History;
    top.ui.history_view_mode = HistoryViewMode::Top;

    let mut playing = AppState::new();
    playing.ui.view = View::NowPlaying;
    playing.domain.current_track = Some(track("playing", "Playing track"));

    let home_history = history(&[
        track("home-old", "Older home track"),
        track("home-new", "Recent home track"),
    ]);
    let mut home = AppState::new();
    home.ui.view = View::Home;
    home.ui.home_section = HomeSection::Recent;
    home.ui.selected_index = 1;

    vec![
        ResolverCase {
            name: "search",
            state: search,
            history: None,
            expected_track_title: "Search result",
            expected_source: TrackSource::Search,
            expected_actions: standard_actions(true),
        },
        ResolverCase {
            name: "queue",
            state: queue,
            history: None,
            expected_track_title: "First queue occurrence",
            expected_source: TrackSource::Queue { order_index: 1 },
            expected_actions: queue_actions,
        },
        ResolverCase {
            name: "playlist detail",
            state: playlist_detail,
            history: None,
            expected_track_title: "Second playlist occurrence",
            expected_source: TrackSource::Playlist {
                playlist_id,
                track_index: 1,
            },
            expected_actions: playlist_actions,
        },
        ResolverCase {
            name: "history recent",
            state: recent,
            history: Some(recent_history),
            expected_track_title: "Recent history track",
            expected_source: TrackSource::History,
            expected_actions: standard_actions(true),
        },
        ResolverCase {
            name: "history top",
            state: top,
            history: Some(top_history),
            expected_track_title: "Top history track",
            expected_source: TrackSource::History,
            expected_actions: standard_actions(true),
        },
        ResolverCase {
            name: "playing",
            state: playing,
            history: None,
            expected_track_title: "Playing track",
            expected_source: TrackSource::Playing,
            expected_actions: standard_actions(true),
        },
        ResolverCase {
            name: "home recent",
            state: home,
            history: Some(home_history),
            expected_track_title: "Older home track",
            expected_source: TrackSource::Home,
            expected_actions: standard_actions(true),
        },
    ]
}

#[test]
fn resolves_current_track_bearing_views_with_exact_source_and_action_order() {
    for case in resolver_cases() {
        let context = resolve_track_context(&case.state, case.history.as_ref())
            .unwrap_or_else(|| panic!("{} should resolve", case.name));

        assert_eq!(
            context.track.title, case.expected_track_title,
            "{}",
            case.name
        );
        assert_eq!(context.source, case.expected_source, "{}", case.name);
        assert_eq!(context.actions, case.expected_actions, "{}", case.name);
    }
}

#[test]
fn hides_add_to_queue_when_video_id_is_already_queued() {
    let selected = track("same-video", "Search selection");
    let mut state = AppState::new();
    state.ui.view = View::Search;
    state.domain.search = SearchState::Results {
        query: "same".to_string(),
        tracks: vec![selected],
    };
    state
        .domain
        .queue
        .push(track("same-video", "Queued occurrence"));

    let context = resolve_track_context(&state, None).expect("search context");

    assert!(!context.actions.contains(&TrackContextAction::AddToQueue));
}

#[test]
fn legacy_track_without_stored_channel_identity_still_offers_visit_channel() {
    let mut selected = track("legacy", "Legacy track");
    selected.channel_id = None;
    selected.channel_url = None;
    let mut state = AppState::new();
    state.ui.view = View::Search;
    state.domain.search = SearchState::Results {
        query: "legacy".to_string(),
        tracks: vec![selected],
    };

    let context = resolve_track_context(&state, None).expect("legacy search context");

    assert!(context.actions.contains(&TrackContextAction::VisitChannel));
}

#[test]
fn populated_channel_resolves_selected_track_without_synthetic_row_fallback() {
    let selected = track("channel-track", "Channel track");
    let mut state = AppState::new();
    state.ui.view = View::Channel;
    state.domain.channel = Some(ChannelState {
        name: "Channel".into(),
        url: "https://www.youtube.com/channel/UC1/videos".into(),
        tracks: vec![selected.clone()],
        next_page: 1,
        exhausted: false,
        loading: false,
        error: None,
        return_to: ChannelNavigationSnapshot {
            view: View::Search,
            focus: Focus::Content,
            selected_index: 0,
        },
        previous: None,
    });

    let context = resolve_track_context(&state, None).expect("channel context");
    assert_eq!(context.track.id, selected.id);
    assert_eq!(context.source, TrackSource::Channel);

    state.ui.selected_index = 1;
    assert!(resolve_track_context(&state, None).is_none());
}
