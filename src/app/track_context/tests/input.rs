use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use crate::app::action::{Action, NavigationAction};
use crate::app::channel::{ChannelNavigationSnapshot, ChannelState};
use crate::app::state::{Focus, HomeSection, View};
use crate::app::tests::test_app;
use crate::media::search::SearchState;
use crate::playback::PlaybackStatus;
use crate::playlists::Playlist;
use crate::playlists::model::PlaylistTrack;

use super::{history, track};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

async fn receive_action(action_rx: &mut mpsc::Receiver<Action>, label: &str) -> Action {
    timeout(Duration::from_millis(100), action_rx.recv())
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {label}"))
        .unwrap_or_else(|| panic!("channel closed before {label}"))
}

async fn open_menu_for(view: View) {
    let (_temp, mut app) = test_app();
    let selected = track("context", "Context track");
    app.state.view = view;
    app.state.playback.status = PlaybackStatus::Playing;
    match view {
        View::Search => {
            app.state.search = SearchState::Results {
                query: "context".to_string(),
                tracks: vec![selected],
            };
        }
        View::Queue => app.state.queue.push(selected),
        View::PlaylistDetail => {
            let mut playlist = Playlist::new("Context playlist");
            playlist.tracks.push(PlaylistTrack::from(&selected));
            app.state.playlists.push(playlist);
            app.state.selected_playlist = Some(0);
        }
        View::History => app.history = Some(history(&[selected])),
        View::NowPlaying => app.state.current_track = Some(selected),
        View::Home => {
            app.state.home_section = HomeSection::Recent;
            app.history = Some(history(&[selected]));
        }
        View::Channel => {
            app.state.channel = Some(ChannelState {
                name: "Context channel".to_string(),
                url: "https://www.youtube.com/channel/UC123".to_string(),
                tracks: vec![selected],
                next_page: 1,
                exhausted: true,
                loading: false,
                error: None,
                return_to: ChannelNavigationSnapshot {
                    view: View::Search,
                    focus: Focus::Content,
                    selected_index: 0,
                },
                previous: None,
            });
        }
        View::Playlists | View::Help => panic!("not a track-bearing view"),
    }
    let (action_tx, mut action_rx) = mpsc::channel(4);

    app.handle_key(key(KeyCode::Char('c')), &action_tx).await;
    assert!(app.state.track_context_menu.is_some(), "view: {view:?}");
    assert!(action_rx.try_recv().is_err(), "opening is atomic");
    assert_eq!(app.state.playback.status, PlaybackStatus::Playing);

    let content_selection = app.state.selected_index;
    app.handle_key(key(KeyCode::Char('j')), &action_tx).await;
    let action = receive_action(&mut action_rx, "move action").await;
    assert!(matches!(
        action,
        Action::Navigation(NavigationAction::MoveTrackContext(1))
    ));
    app.handle_action(action, &action_tx).await;
    assert_eq!(
        app.state
            .track_context_menu
            .as_ref()
            .map(|menu| menu.selected),
        Some(1)
    );
    assert_eq!(app.state.selected_index, content_selection);
    assert_eq!(app.state.playback.status, PlaybackStatus::Playing);

    app.handle_key(key(KeyCode::Char('k')), &action_tx).await;
    let action = receive_action(&mut action_rx, "move action").await;
    app.handle_action(action, &action_tx).await;
    assert_eq!(
        app.state
            .track_context_menu
            .as_ref()
            .map(|menu| menu.selected),
        Some(0)
    );

    app.handle_key(key(KeyCode::Esc), &action_tx).await;
    let action = receive_action(&mut action_rx, "close action").await;
    assert!(matches!(
        action,
        Action::Navigation(NavigationAction::CloseTrackContext)
    ));
    app.handle_action(action, &action_tx).await;
    assert!(app.state.track_context_menu.is_none());
    assert_eq!(app.state.playback.status, PlaybackStatus::Playing);
}

#[tokio::test]
async fn track_context_menu_keys_work_on_every_track_bearing_view() {
    for view in [
        View::Search,
        View::Queue,
        View::PlaylistDetail,
        View::History,
        View::NowPlaying,
        View::Home,
    ] {
        timeout(Duration::from_secs(1), open_menu_for(view))
            .await
            .unwrap_or_else(|_| panic!("context-menu input lifecycle hung for {view:?}"));
    }
}

#[tokio::test]
async fn track_context_menu_swallows_unhandled_global_keys() {
    let (_temp, mut app) = test_app();
    app.state.view = View::Search;
    app.state.search = SearchState::Results {
        query: "context".to_string(),
        tracks: vec![track("context", "Context track")],
    };
    let context =
        crate::app::track_context::resolve_track_context(&app.state, None).expect("track context");
    app.state.track_context_menu = Some(crate::app::state::TrackContextMenuState {
        context,
        selected: 0,
    });
    let (action_tx, mut action_rx) = mpsc::channel(1);

    app.handle_key(key(KeyCode::Char('q')), &action_tx).await;

    assert!(action_rx.try_recv().is_err());
    assert!(app.state.running);
    assert!(app.state.track_context_menu.is_some());
}

#[tokio::test]
async fn track_context_menu_details_modal_is_modal_first_and_closes_with_escape() {
    let (_temp, mut app) = test_app();
    app.state.track_details_modal = Some(crate::app::state::TrackDetailsModalState {
        track: track("details", "Details track"),
        details: None,
    });
    let (action_tx, mut action_rx) = mpsc::channel(2);

    app.handle_key(key(KeyCode::Char('q')), &action_tx).await;
    assert!(action_rx.try_recv().is_err());
    assert!(app.state.running);

    app.handle_key(key(KeyCode::Esc), &action_tx).await;
    let action = receive_action(&mut action_rx, "close details action").await;
    assert!(matches!(
        action,
        Action::Navigation(NavigationAction::CloseTrackDetails)
    ));
    app.handle_action(action, &action_tx).await;
    assert!(app.state.track_details_modal.is_none());
}
