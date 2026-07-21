use super::*;

fn activate_transition(state: &mut AppState) {
    let now = std::time::Instant::now();
    state.track_transition.update(
        ytm_tui::playback::TransitionInput {
            occurrence: Some(1),
            remaining_seconds: Some(15.0),
            playing: true,
            has_next: true,
        },
        now - ytm_tui::playback::TRANSITION_DURATION,
    );
}

#[test]
fn playback_transition_replaces_title_in_shared_player() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::NowPlaying;
    let current = Track::new("current", "Current title", "Current channel");
    state.current_track = Some(current.clone());
    state.queue.push(current);
    state
        .queue
        .push(Track::new("next", "Next title", "Next channel"));
    state.queue.position = Some(0);
    activate_transition(&mut state);

    let out = render_to_string(&mut state, None, 80, 24);
    assert!(
        out.contains("Current title < Next title"),
        "transition title:\n{out}"
    );
    assert!(
        !out.contains("Current title — Current channel"),
        "normal title replaced:\n{out}"
    );
}

#[test]
fn playback_transition_is_width_safe_and_sanitized() {
    let mut state = AppState::new();
    let current = Track::new("current", "Current\u{1b}[2J 世界 title", "Channel");
    state.current_track = Some(current.clone());
    state.queue.push(current);
    state.queue.push(Track::new(
        "next",
        "Next 🎵 title with deliberately excessive width",
        "Next channel",
    ));
    state.queue.position = Some(0);
    activate_transition(&mut state);

    for width in [60, 80] {
        let out = render_to_string(&mut state, None, width, 24);
        assert!(out.contains("Current[2J"), "sanitized:\n{out}");
        assert!(!out.contains('\u{1b}'), "control removed:\n{out}");
        assert!(out.contains(" < "), "visible separator at {width}:\n{out}");
        assert!(
            out.lines()
                .all(|line| line.chars().count() == width as usize),
            "buffer remains exactly {width} cells wide:\n{out}"
        );
    }
}

#[test]
fn playback_transition_requires_a_real_next_track() {
    for repeat_track in [false, true] {
        let mut state = AppState::new();
        state.view = ytm_tui::app::state::View::NowPlaying;
        let current = Track::new("current", "Regular title", "Channel");
        state.current_track = Some(current.clone());
        state.queue.push(current);
        if repeat_track {
            state
                .queue
                .push(Track::new("next", "Suppressed next", "Channel"));
            state.queue.repeat = ytm_tui::queue::RepeatMode::Track;
        }
        state.queue.position = Some(0);
        activate_transition(&mut state);

        let out = render_to_string(&mut state, None, 80, 24);
        assert!(
            out.contains("Regular title — Channel"),
            "normal title:\n{out}"
        );
        assert!(!out.contains(" < "), "transition suppressed:\n{out}");
    }
}

#[test]
fn playing_view_shows_chapters_and_up_next() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::NowPlaying;
    let mut track = Track::new("mix1", "Essential Mix", "Skee Mask");
    track.duration_seconds = Some(7115);
    state.current_track = Some(track.clone());
    state.queue.push(track);
    state
        .queue
        .push(Track::new("next1", "Next Song", "Other Artist"));
    state.queue.position = Some(0);
    state.playback.status = ytm_tui::playback::PlaybackStatus::Playing;
    state.playback.position_seconds = 240.0;
    state.playback.duration_seconds = Some(7115.0);
    state.current_details = Some(ytm_tui::media::TrackDetails {
        description: Some("Tracklist:\n0:00 Intro\n3:45 Second Tune\n10:00 Third Tune".to_string()),
        chapters: ytm_tui::media::parse_chapters_from_description(
            "0:00 Intro\n3:45 Second Tune\n10:00 Third Tune",
        ),
        ..Default::default()
    });
    let out = render_to_string(&mut state, None, 120, 34);
    println!("\n{out}");
    assert!(out.contains("TRACKLIST (3)"), "chapter count:\n{out}");
    assert!(out.contains("Second Tune"), "chapter titles:\n{out}");
    assert!(out.contains("UP NEXT"), "up next pane:\n{out}");
    assert!(out.contains("Next Song"), "upcoming track:\n{out}");
    assert!(!out.contains("Shuffle:"), "mode row removed:\n{out}");
    assert!(!out.contains("vol 0%"), "volume row removed:\n{out}");
    // Playback information appears once, in the shared bottom bar.
    assert_eq!(
        out.matches("04:00 / 1:58:35").count(),
        1,
        "single gauge:\n{out}"
    );
}

#[test]
fn playing_layout_exercises_all_four_breakpoints() {
    let render = |width: u16, height: u16| {
        let mut state = AppState::new();
        state.view = ytm_tui::app::state::View::NowPlaying;
        let mut track = Track::new("mix", "Width-safe title", "Artist");
        track.duration_seconds = Some(600);
        state.current_track = Some(track.clone());
        state.queue.push(track);
        state
            .queue
            .push(Track::new("next", "Next width-safe", "Other"));
        state.queue.position = Some(0);
        state.playback.duration_seconds = Some(600.0);
        state.current_details = Some(ytm_tui::media::TrackDetails {
            description: Some("A useful description".to_string()),
            uploader: Some("Verified channel".to_string()),
            upload_date: Some("20210515".to_string()),
            acodec: Some("opus".to_string()),
            abr: Some(128.0),
            asr: Some(48_000),
            audio_channels: Some(2),
            ..Default::default()
        });
        render_to_string(&mut state, None, width, height)
    };

    let narrow = render(80, 24);
    assert!(narrow.contains("UP NEXT"), "narrow stack:\n{narrow}");
    assert!(
        !narrow.contains("METADATA"),
        "narrow metadata hidden:\n{narrow}"
    );

    let medium = render(120, 36);
    assert!(medium.contains("DESCRIPTION"), "medium detail:\n{medium}");
    let panel_header = medium
        .lines()
        .find(|line| line.contains("DESCRIPTION") && line.contains("UP NEXT"))
        .expect("side-by-side panel header");
    assert!(
        panel_header.find("DESCRIPTION") < panel_header.find("UP NEXT"),
        "description appears left of up next:\n{medium}"
    );
    assert!(
        !medium.contains("METADATA"),
        "medium metadata hidden:\n{medium}"
    );

    let wide = render(150, 40);
    assert!(wide.contains("METADATA"), "wide metadata:\n{wide}");
    assert!(wide.contains("BITRATE"), "wide bitrate:\n{wide}");
    assert!(wide.contains("[OPUS]"), "conditional chips:\n{wide}");

    let ultra = render(180, 48);
    assert!(ultra.contains("QUEUE"), "ultra queue:\n{ultra}");
    assert!(
        !ultra.contains("ACTIVITY"),
        "activity belongs on Home:\n{ultra}"
    );
    assert_eq!(
        ultra.matches("DESCRIPTION").count(),
        1,
        "single expanded description panel:\n{ultra}"
    );
}

#[test]
fn playing_ultra_wide_does_not_render_persisted_activity() {
    use ytm_tui::history::activity::{ActivityEvent, ActivityKind};

    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::NowPlaying;
    state.current_track = Some(Track::new("playing", "Playing", "Artist"));
    state.activity.push(ActivityEvent::new(
        ActivityKind::PlaylistImported,
        "Imported activity",
        "12 tracks",
    ));
    let out = render_to_string(&mut state, None, 180, 48);
    assert!(!out.contains("ACTIVITY"), "activity panel removed:\n{out}");
    assert!(
        !out.contains("Imported activity"),
        "activity content hidden:\n{out}"
    );
    assert!(out.contains("DESCRIPTION"), "description retained:\n{out}");
}

#[test]
fn playing_layout_omits_unknown_format_chips() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::NowPlaying;
    state.current_track = Some(Track::new("id", "No details", "Artist"));
    state.current_details = Some(ytm_tui::media::TrackDetails::default());
    let out = render_to_string(&mut state, None, 150, 40);
    for unsupported in ["kbps", "Hz", "Stereo"] {
        assert!(
            !out.contains(unsupported),
            "fabricated {unsupported}:\n{out}"
        );
    }
}
