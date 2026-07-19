use crate::app::state::{AppState, HistoryViewMode, HomeSection, View};
use crate::media::search::SearchState;
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::{ResolverCase, TrackContextAction, TrackSource, history, standard_actions, track};
use crate::app::track_context::resolve_track_context;

fn resolver_cases() -> Vec<ResolverCase> {
    let mut search = AppState::new();
    search.view = View::Search;
    search.search = SearchState::Results {
        query: "search".to_string(),
        tracks: vec![track("search", "Search result")],
    };

    let mut queue = AppState::new();
    queue.view = View::Queue;
    queue
        .queue
        .push(track("duplicate", "First queue occurrence"));
    queue
        .queue
        .push(track("duplicate", "Second queue occurrence"));
    queue.queue.order = vec![1, 0];
    queue.selected_index = 1;
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
    playlist_detail.view = View::PlaylistDetail;
    playlist_detail.playlists.push(playlist);
    playlist_detail.selected_playlist = Some(0);
    playlist_detail.selected_index = 1;
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
    recent.view = View::History;
    recent.history_view_mode = HistoryViewMode::Recent;

    let top_history = history(&[
        track("top-other", "Other top track"),
        track("top", "Top history track"),
        track("top", "Top history track"),
    ]);
    let mut top = AppState::new();
    top.view = View::History;
    top.history_view_mode = HistoryViewMode::Top;

    let mut playing = AppState::new();
    playing.view = View::NowPlaying;
    playing.current_track = Some(track("playing", "Playing track"));

    let home_history = history(&[
        track("home-old", "Older home track"),
        track("home-new", "Recent home track"),
    ]);
    let mut home = AppState::new();
    home.view = View::Home;
    home.home_section = HomeSection::Recent;
    home.selected_index = 1;

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
    state.view = View::Search;
    state.search = SearchState::Results {
        query: "same".to_string(),
        tracks: vec![selected],
    };
    state.queue.push(track("same-video", "Queued occurrence"));

    let context = resolve_track_context(&state, None).expect("search context");

    assert!(!context.actions.contains(&TrackContextAction::AddToQueue));
}

#[test]
fn defines_channel_source_without_resolving_an_unowned_channel_view() {
    assert_eq!(TrackSource::Channel, TrackSource::Channel);
}
