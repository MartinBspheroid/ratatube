use super::*;
use ratatube::app::channel::{ChannelNavigationSnapshot, ChannelState};
use ratatube::app::state::{Focus, View};

fn channel_state(tracks: Vec<Track>) -> AppState {
    let mut state = AppState::new();
    state.ui.view = View::Channel;
    state.domain.channel = Some(ChannelState {
        name: "Remote\u{1b}[2J Channel".into(),
        url: "https://www.youtube.com/channel/UC1/videos".into(),
        tracks,
        next_page: 1,
        exhausted: false,
        loading: false,
        error: None,
        return_to: ChannelNavigationSnapshot {
            view: View::Search,
            focus: Focus::Content,
            selected_index: 2,
        },
        previous: None,
    });
    state
}

#[test]
fn channel_states_are_truthful_and_width_safe() {
    for width in [80, 120, 150, 180] {
        let mut loading = channel_state(Vec::new());
        loading.domain.channel.as_mut().expect("channel").loading = true;
        let out = render_to_string(&mut loading, None, width, 30);
        assert!(out.contains("Loading"), "loading at {width}:\n{out}");
        assert!(!out.contains('\u{1b}'), "remote controls at {width}");

        let mut failed = channel_state(Vec::new());
        failed.domain.channel.as_mut().expect("channel").error = Some("offline".into());
        let out = render_to_string(&mut failed, None, width, 30);
        assert!(out.contains("Retry"), "retry at {width}:\n{out}");
        assert!(out.contains("offline"), "error detail at {width}:\n{out}");

        let mut empty = channel_state(Vec::new());
        empty.domain.channel.as_mut().expect("channel").exhausted = true;
        let out = render_to_string(&mut empty, None, width, 30);
        assert!(out.contains("no public videos"), "empty at {width}:\n{out}");
    }
}

#[test]
fn populated_channel_has_explicit_pagination_and_responsive_preview() {
    let track = Track::new("video", "Newest video", "Actual channel");
    for width in [80, 120, 150, 180] {
        let mut state = channel_state(vec![track.clone()]);
        let out = render_to_string(&mut state, None, width, 30);
        assert!(out.contains("Newest video"), "track at {width}:\n{out}");
        assert!(out.contains("Load more"), "pagination at {width}:\n{out}");
        if width == 80 {
            assert!(
                !out.contains("Selection"),
                "narrow preview at {width}:\n{out}"
            );
        } else {
            assert!(out.contains("SELECTION"), "wide preview at {width}:\n{out}");
        }
    }
}

#[test]
fn exhausted_channel_has_no_synthetic_row() {
    let mut state = channel_state(vec![Track::new("video", "Only video", "Channel")]);
    state.domain.channel.as_mut().expect("channel").exhausted = true;
    let out = render_to_string(&mut state, None, 120, 30);
    assert!(!out.contains("Load more"), "exhausted:\n{out}");
    assert!(!out.contains("Retry"), "exhausted:\n{out}");
}
