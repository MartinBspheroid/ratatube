//! Pane-aware click targeting and Enter-parity double-clicks.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use tokio::sync::mpsc;

use super::test_app;
use crate::app::action::{Action, NavigationAction, PlaybackAction, PlaylistAction};
use crate::app::state::{Focus, HomeHitZone, HomeSection, PlayingPane, View};
use crate::media::Track;
use crate::playlists::Playlist;

fn click(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn track(id: &str) -> Track {
    Track::new(id, id, "artist")
}

#[tokio::test]
async fn playlists_double_click_opens_the_playlist_like_enter() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Playlists;
    app.state.domain.playlists = vec![
        Playlist::new("First"),
        Playlist::new("Second"),
        Playlist::new("Third"),
    ];
    app.state.ui.list_hit_area = Rect::new(2, 4, 40, 6);
    let (action_tx, mut action_rx) = mpsc::channel(4);

    let event = click(5, 5);
    app.handle_mouse(event, &action_tx).await;
    assert_eq!(app.state.ui.selected_index, 1);
    assert!(action_rx.try_recv().is_err(), "single click only selects");

    app.handle_mouse(event, &action_tx).await;
    assert!(
        matches!(
            action_rx.try_recv(),
            Ok(Action::Playlists(PlaylistAction::OpenPlaylistDetail))
        ),
        "double-click must perform the view's Enter action"
    );
}

#[tokio::test]
async fn home_click_focuses_the_section_and_double_click_plays() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Home;
    app.state.ui.home_section = HomeSection::Recent;
    app.state.ui.home_recent_len = 3;
    app.state.domain.playlists = vec![Playlist::new("Mix")];
    app.state.ui.home_hit_zones = vec![
        HomeHitZone {
            section: HomeSection::Recent,
            index: 1,
            area: Rect::new(2, 5, 30, 1),
        },
        HomeHitZone {
            section: HomeSection::Playlists,
            index: 0,
            area: Rect::new(2, 12, 30, 3),
        },
    ];
    let (action_tx, mut action_rx) = mpsc::channel(4);

    // Clicking another section's item switches the section first.
    app.handle_mouse(click(4, 13), &action_tx).await;
    assert_eq!(app.state.ui.home_section, HomeSection::Playlists);
    assert_eq!(app.state.ui.selected_index, 0);
    assert!(action_rx.try_recv().is_err());

    // A second click on the same item acts as Enter (global PlaySelected).
    app.handle_mouse(click(4, 13), &action_tx).await;
    assert!(matches!(
        action_rx.try_recv(),
        Ok(Action::Playback(PlaybackAction::PlaySelected))
    ));

    // Section-switching clicks never count toward a cross-pane double-click.
    app.handle_mouse(click(4, 5), &action_tx).await;
    assert_eq!(app.state.ui.home_section, HomeSection::Recent);
    assert_eq!(app.state.ui.selected_index, 1);
    assert!(action_rx.try_recv().is_err());
}

#[tokio::test]
async fn playing_queue_pane_clicks_use_the_rendered_window_offset() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::NowPlaying;
    app.state.ui.playing_pane = PlayingPane::Info;
    // The queue pane only exists on the ultra-wide layout.
    app.state.ui.screen_area = Rect::new(0, 0, 180, 50);
    for index in 0..20 {
        app.state.domain.queue.push(track(&format!("q{index}")));
    }
    app.state.ui.list_hit_area = Rect::new(50, 6, 30, 8);
    // The pane windows rows manually from item 9; the stale list state
    // offset would otherwise map this click to item 2.
    app.state.ui.list_hit_offset = Some(9);
    *app.state.ui.list_state.offset_mut() = 0;
    let (action_tx, _action_rx) = mpsc::channel(4);

    app.handle_mouse(click(55, 8), &action_tx).await;
    assert_eq!(app.state.ui.selected_index, 11);
    assert_eq!(
        app.state.ui.playing_pane,
        PlayingPane::Queue,
        "clicking a queue row focuses the queue pane"
    );
}

#[tokio::test]
async fn clicking_a_result_leaves_search_input_focus_before_enter_fires() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Search;
    app.state.ui.focus = Focus::SearchInput;
    app.state.ui.search_input = "unfinished query".to_string();
    app.state.domain.search = crate::media::search::SearchState::Results {
        query: "done".to_string(),
        tracks: vec![track("a"), track("b")],
    };
    app.state.ui.list_hit_area = Rect::new(2, 4, 40, 6);
    let (action_tx, mut action_rx) = mpsc::channel(4);

    let event = click(5, 4);
    app.handle_mouse(event, &action_tx).await;
    assert_eq!(app.state.ui.focus, Focus::Content);
    app.handle_mouse(event, &action_tx).await;
    assert!(
        matches!(
            action_rx.try_recv(),
            Ok(Action::Playback(PlaybackAction::PlaySelected))
        ),
        "double-click plays the result instead of submitting the input"
    );
}

#[tokio::test]
async fn scrolled_queue_clicks_map_rows_through_the_table_offset() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Queue;
    for index in 0..30 {
        app.state.domain.queue.push(track(&format!("q{index}")));
    }
    app.state.ui.list_hit_area = Rect::new(2, 4, 40, 10);
    *app.state.ui.table_state.offset_mut() = 12;
    // A stale list-state offset from another view must not leak in.
    *app.state.ui.list_state.offset_mut() = 3;
    let (action_tx, mut action_rx) = mpsc::channel(4);

    let event = click(5, 6);
    app.handle_mouse(event, &action_tx).await;
    assert_eq!(app.state.ui.selected_index, 14);

    app.handle_mouse(event, &action_tx).await;
    assert!(matches!(
        action_rx.try_recv(),
        Ok(Action::Playback(PlaybackAction::PlaySelected))
    ));
}

#[tokio::test]
async fn clicking_the_timeline_row_seeks_by_fraction() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Queue;
    app.state.domain.current_track = Some(track("playing"));
    app.state.ui.screen_area = Rect::new(0, 0, 100, 30);
    let layout = crate::ui::layout::AppLayout::new(app.state.ui.screen_area, true, true);
    let bar = layout.now_playing;
    assert!(bar.height >= 4, "fixture terminal renders the bar");
    let (action_tx, mut action_rx) = mpsc::channel(4);

    app.handle_mouse(click(bar.x + bar.width / 2, bar.y + 2), &action_tx)
        .await;
    let Ok(Action::Playback(PlaybackAction::SeekToFraction(fraction))) = action_rx.try_recv()
    else {
        panic!("timeline click must seek");
    };
    assert!((fraction - 0.5).abs() < 0.02, "{fraction}");
}

#[tokio::test]
async fn channel_double_click_on_the_load_more_row_loads_more() {
    let (_temp, mut app) = test_app();
    app.state.ui.view = View::Channel;
    app.state.domain.channel = Some(crate::app::channel::ChannelState {
        name: "Channel".into(),
        url: "https://www.youtube.com/channel/UC1/videos".into(),
        tracks: (0..3)
            .map(|index| track(&format!("track-{index}")))
            .collect(),
        next_page: 1,
        exhausted: false,
        loading: false,
        error: None,
        return_to: crate::app::channel::ChannelNavigationSnapshot {
            view: View::Search,
            focus: Focus::Content,
            selected_index: 0,
        },
        previous: None,
    });
    app.state.ui.list_hit_area = Rect::new(2, 4, 40, 6);
    let (action_tx, mut action_rx) = mpsc::channel(4);

    // Row 3 is the trailing "Load more" row (one past the tracks).
    let event = click(5, 7);
    app.handle_mouse(event, &action_tx).await;
    assert_eq!(app.state.ui.selected_index, 3);
    app.handle_mouse(event, &action_tx).await;
    assert!(
        matches!(
            action_rx.try_recv(),
            Ok(Action::Navigation(NavigationAction::LoadMoreChannel))
        ),
        "Enter parity includes the load-more special case"
    );
}
