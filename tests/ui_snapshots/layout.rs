use super::*;

#[test]
fn now_playing_bar_renders_when_track_loaded() {
    let mut state = AppState::new();
    state.domain.current_track = Some(Track::new("abc", "Teardrop", "Massive Attack"));
    state.domain.playback.status = ratatube::playback::PlaybackStatus::Playing;
    state.domain.playback.position_seconds = 161.0;
    state.domain.playback.duration_seconds = Some(330.0);
    state.domain.playback.volume = 72;
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Teardrop"), "title:\n{out}");
    assert!(out.contains("02:41 / 05:30"), "elapsed/total times:\n{out}");
    assert!(out.contains("72%"), "volume:\n{out}");
    assert!(!out.contains("NOW PLAYING"), "badge removed:\n{out}");
    assert!(!out.contains("< | >"), "transport cluster removed:\n{out}");
    assert!(out.contains("[SHUFFLE]"), "shuffle control:\n{out}");
    assert!(out.contains("[REPEAT]"), "repeat control:\n{out}");
    assert!(out.contains("[VOL] =====___ 72%"), "volume gauge:\n{out}");
}

#[test]
fn header_clusters_follow_collapse_boundaries() {
    let mut state = AppState::new();
    state.domain.mpv_ready = true;
    state.domain.yt_dlp_ready = true;
    state.domain.playback.volume = 58;
    state.domain.queue.push(Track::new("q", "Queued", "Artist"));

    let collapsed = render_to_string(&mut state, None, 99, 24);
    assert!(
        collapsed.contains("ratatube v0.1.0"),
        "real version:\n{collapsed}"
    );
    assert!(
        !collapsed
            .lines()
            .next()
            .unwrap_or_default()
            .contains("vol 58%")
    );

    let volume = render_to_string(&mut state, None, 100, 24);
    let volume_header = volume.lines().next().unwrap_or_default();
    assert!(
        !volume_header.contains("vol 58%"),
        "volume text removed:\n{volume}"
    );
    assert!(
        !volume_header.contains("queue 1"),
        "queue still collapsed:\n{volume}"
    );

    let medium = render_to_string(&mut state, None, 139, 24);
    assert!(
        !medium
            .lines()
            .next()
            .unwrap_or_default()
            .contains("queue 1"),
        "queue remains collapsed at medium boundary:\n{medium}"
    );

    let full = render_to_string(&mut state, None, 140, 24);
    let full_header = full.lines().next().unwrap_or_default();
    assert!(
        !full_header.contains("vol 58%"),
        "volume text removed:\n{full}"
    );
    assert!(full_header.contains("queue 1"), "queue cluster:\n{full}");
    assert!(!full.contains("Online"), "no fabricated status:\n{full}");
}

#[test]
fn layouts_render_width_safe_content_at_every_breakpoint_edge() {
    for width in [99, 100, 139, 140, 169, 170] {
        let mut state = AppState::new();
        state.ui.view = ratatube::app::state::View::Search;
        state.domain.search = SearchState::Results {
            query: "boundary".to_string(),
            tracks: vec![Track::new("edge", "Boundary stem", "Edge artist")],
        };
        let out = render_to_string(&mut state, None, width, 30);
        assert!(
            out.contains("Boundary stem"),
            "width {width} lost stable content:\n{out}"
        );
    }
}

#[test]
fn narrow_header_keeps_active_tab_label() {
    let mut state = AppState::new();
    state.ui.view = ratatube::app::state::View::Search;
    let out = render_to_string(&mut state, None, 80, 24);
    assert!(
        out.lines()
            .next()
            .unwrap_or_default()
            .contains("[SEARCH] Search"),
        "active label:\n{out}"
    );
}

#[test]
fn footers_never_advertise_rejected_unbound_features() {
    for view in ratatube::app::state::View::TABS {
        let mut state = AppState::new();
        state.ui.view = view;
        let out = render_to_string(&mut state, None, 100, 24);
        let footer = out.lines().last().unwrap_or_default();
        for rejected in ["Sort", "Filters", "Refresh", "Library", "Settings"] {
            assert!(
                !footer.contains(rejected),
                "{view:?} advertises {rejected}:\n{out}"
            );
        }
    }
}

#[test]
fn tiny_terminal_shows_compact_warning() {
    let mut state = AppState::new();
    let out = render_to_string(&mut state, None, 30, 8);
    assert!(
        out.contains("Terminal too small"),
        "compact warning:\n{out}"
    );
}

#[test]
fn small_terminal_still_renders_ui() {
    let mut state = AppState::new();
    // Below the documented 80x24 minimum but above the hard floor.
    let out = render_to_string(&mut state, None, 60, 20);
    assert!(out.contains("Srch"), "narrow tabs still render:\n{out}");
    assert!(out.contains("small terminal"), "size note:\n{out}");
}
