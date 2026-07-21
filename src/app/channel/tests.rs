use crate::app::state::{Focus, View};
use crate::media::Track;
use crate::media::channel::ChannelPage;
use crate::media::yt_dlp::ImportRejections;

use super::{ChannelNavigationSnapshot, ChannelState};
use crate::app::tests::test_app;

fn snapshot() -> ChannelNavigationSnapshot {
    ChannelNavigationSnapshot {
        view: View::Search,
        focus: Focus::Content,
        selected_index: 3,
    }
}

fn state() -> ChannelState {
    ChannelState::new(
        &Track::new("source", "Source", "Channel"),
        "https://www.youtube.com/channel/UC1/videos".into(),
        snapshot(),
    )
}

fn page(ids: &[&str], exhausted: bool) -> ChannelPage {
    ChannelPage {
        tracks: ids
            .iter()
            .map(|id| Track::new(*id, format!("Title {id}"), "Channel"))
            .collect(),
        rejections: ImportRejections::default(),
        exhausted,
    }
}

#[test]
fn initial_page_preserves_newest_order_and_advances_once() {
    let mut channel = state();
    channel.loading = true;
    channel.append(page(&["new", "old"], false));
    assert_eq!(
        channel
            .tracks
            .iter()
            .map(|track| track.id.as_str())
            .collect::<Vec<_>>(),
        ["new", "old"]
    );
    assert_eq!(channel.next_page, 1);
    assert!(!channel.loading);
}

#[test]
fn later_pages_deduplicate_without_reordering_existing_rows() {
    let mut channel = state();
    channel.append(page(&["new", "middle"], false));
    channel.append(page(&["middle", "old"], true));
    assert_eq!(
        channel
            .tracks
            .iter()
            .map(|track| track.id.as_str())
            .collect::<Vec<_>>(),
        ["new", "middle", "old"]
    );
    assert!(channel.exhausted);
}

#[test]
fn failed_page_can_retry_same_page_without_losing_rows() {
    let mut channel = state();
    channel.append(page(&["new"], false));
    channel.loading = false;
    channel.error = Some("offline".into());
    assert_eq!(channel.next_page, 1);
    assert_eq!(channel.tracks[0].id, "new");
    assert!(!channel.exhausted);
}

#[test]
fn return_snapshot_keeps_view_focus_and_selection() {
    assert_eq!(state().return_to, snapshot());
}

#[test]
fn stale_channel_or_page_completion_is_ignored() {
    let (_temp, mut app) = test_app();
    app.state.domain.channel = Some(state());
    app.finish_channel_page(
        "https://www.youtube.com/channel/OTHER/videos",
        0,
        Ok(page(&["wrong"], false)),
    );
    app.finish_channel_page(
        "https://www.youtube.com/channel/UC1/videos",
        7,
        Ok(page(&["wrong-page"], false)),
    );
    assert!(
        app.state
            .domain
            .channel
            .as_ref()
            .expect("channel")
            .tracks
            .is_empty()
    );
}

#[test]
fn later_failure_preserves_rows_and_back_restores_navigation() {
    let (_temp, mut app) = test_app();
    let mut channel = state();
    channel.append(page(&["new"], false));
    channel.loading = true;
    app.state.domain.channel = Some(channel);
    app.state.ui.view = View::Channel;
    app.finish_channel_page(
        "https://www.youtube.com/channel/UC1/videos",
        1,
        Err("offline".into()),
    );
    let channel = app.state.domain.channel.as_ref().expect("channel");
    assert_eq!(channel.tracks[0].id, "new");
    assert_eq!(channel.next_page, 1);
    assert_eq!(channel.error.as_deref(), Some("offline"));

    app.leave_channel();
    assert_eq!(app.state.ui.view, View::Search);
    assert_eq!(app.state.ui.focus, Focus::Content);
    assert_eq!(app.state.ui.selected_index, 3);
}

#[tokio::test]
async fn nested_channel_back_restores_original_channel_and_selection() {
    let (_temp, mut app) = test_app();
    let mut original = state();
    original.append(page(&["first", "selected"], true));
    original.loading = true;
    app.state.domain.channel = Some(original);
    app.state.ui.view = View::Channel;
    app.state.ui.selected_index = 1;
    let mut nested_track = Track::new("nested", "Nested", "Other channel");
    nested_track.channel_url = Some("https://www.youtube.com/channel/UC2".into());
    let (action_tx, _action_rx) = tokio::sync::mpsc::channel(4);

    app.open_channel(
        nested_track,
        "https://www.youtube.com/channel/UC2".into(),
        action_tx,
    );
    assert_eq!(
        app.state.domain.channel.as_ref().expect("nested").name,
        "Other channel"
    );

    app.leave_channel();

    let restored = app.state.domain.channel.as_ref().expect("original channel");
    assert_eq!(restored.url, "https://www.youtube.com/channel/UC1/videos");
    assert_eq!(restored.tracks[1].id, "selected");
    assert!(
        !restored.loading,
        "cancelled page must not restore a stale spinner"
    );
    assert_eq!(app.state.ui.view, View::Channel);
    assert_eq!(app.state.ui.selected_index, 1);
}
