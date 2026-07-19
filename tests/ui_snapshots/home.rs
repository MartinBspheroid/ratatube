use super::*;

#[test]
fn home_is_the_landing_view() {
    let mut state = AppState::new();
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Welcome to ytm"), "first-run hero:\n{out}");
    assert!(
        out.contains("Press / to search · 4 for playlists · ? for help"),
        "actionable onboarding:\n{out}"
    );
}

#[test]
fn home_shows_armed_resume_card() {
    let mut state = AppState::new();
    let mut track = Track::new("abc", "BBC Essential Mix", "Skee Mask");
    track.duration_seconds = Some(7115);
    state.current_track = Some(track.clone());
    state.pending_resume = Some(ytm_tui::app::state::PendingResume {
        track,
        position_seconds: 3731.0,
        armed: true,
        play_on_load: false,
    });
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Skee Mask"), "artist:\n{out}");
    assert!(out.contains("1:02:11 / 1:58:35"), "position:\n{out}");
    assert!(out.contains("Space resume"), "armed hint:\n{out}");
}

#[test]
fn home_hides_quick_resume_after_playback_has_moved_on() {
    let pending_track = Track::new("resume", "Old session", "Artist");
    for (current, status) in [
        (
            Track::new("different", "Currently playing", "Artist"),
            ytm_tui::playback::PlaybackStatus::Playing,
        ),
        (
            pending_track.clone(),
            ytm_tui::playback::PlaybackStatus::Playing,
        ),
    ] {
        let mut state = AppState::new();
        state.current_track = Some(current);
        state.playback.status = status;
        state.pending_resume = Some(ytm_tui::app::state::PendingResume {
            track: pending_track.clone(),
            position_seconds: 30.0,
            armed: true,
            play_on_load: false,
        });
        state
            .playlists
            .push(ytm_tui::playlists::Playlist::new("Kept dashboard"));
        let out = render_to_string(&mut state, None, 150, 40);
        assert!(!out.contains("QUICK RESUME"), "stale resume hidden:\n{out}");
        assert!(out.contains("PLAYLISTS"), "dashboard retained:\n{out}");
    }
}

#[test]
fn home_dashboard_exercises_all_four_breakpoints() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut history =
        HistoryService::load(&dir.path().join("home-history.json"), 500).expect("history");
    let mut recent = Track::new("recent", "Recent width-safe", "Recent artist");
    recent.duration_seconds = Some(245);
    history.record(HistoryEntry::from_track(
        &recent,
        None,
        PlaybackOutcome::Stopped,
        120,
    ));

    let render = |width: u16, height: u16| {
        let mut state = AppState::new();
        let mut resume = Track::new("resume", "Resume width-safe", "Resume artist");
        resume.duration_seconds = Some(600);
        state.pending_resume = Some(ytm_tui::app::state::PendingResume {
            track: resume,
            position_seconds: 120.0,
            armed: true,
            play_on_load: false,
        });
        let mut playlist = ytm_tui::playlists::Playlist::new("Playlist width-safe");
        playlist
            .tracks
            .push(ytm_tui::playlists::model::PlaylistTrack::from(&recent));
        state.playlists.push(playlist);
        render_to_string(&mut state, Some(&history), width, height)
    };

    let narrow = render(80, 24);
    assert!(narrow.contains("QUICK RESUME"), "narrow resume:\n{narrow}");
    assert!(narrow.contains("RECENT TRACKS"), "narrow recent:\n{narrow}");
    assert!(!narrow.contains("CONTINUE LISTENING"), "removed:\n{narrow}");
    assert!(!narrow.contains("ACTIVITY"), "removed:\n{narrow}");

    for (width, height) in [(120, 36), (150, 40), (180, 48)] {
        let out = render(width, height);
        for panel in ["QUICK RESUME", "RECENT TRACKS", "PLAYLISTS"] {
            assert!(
                out.contains(panel),
                "{width}x{height} missing {panel}:\n{out}"
            );
        }
        assert!(!out.contains("CONTINUE LISTENING"), "removed:\n{out}");
        assert!(!out.contains("ACTIVITY"), "removed:\n{out}");
        assert!(out.contains("Last played"), "history timestamp:\n{out}");
        assert!(out.contains("View all"), "real navigation links:\n{out}");
    }
}

#[test]
fn home_playlist_tiles_page_with_semantic_chevrons() {
    let mut state = AppState::new();
    state.home_section = ytm_tui::app::state::HomeSection::Playlists;
    state.selected_index = 4;
    state.playlists = (1..=5)
        .map(|index| ytm_tui::playlists::Playlist::new(format!("Paged playlist {index}")))
        .collect();
    let out = render_to_string(&mut state, None, 220, 48);
    assert!(out.contains("< 2/2 >"), "ASCII chevron pager:\n{out}");
    assert!(out.contains("Paged playlist 5"), "selected page:\n{out}");
    assert!(
        !out.contains("Paged playlist 1"),
        "previous page hidden:\n{out}"
    );
}

#[test]
fn home_omits_activity_and_continue_listening_data() {
    use ytm_tui::history::activity::{ActivityEvent, ActivityKind};

    let dir = tempfile::tempdir().expect("tempdir");
    let mut history =
        HistoryService::load(&dir.path().join("resume-history.json"), 500).expect("history");
    let mut continued = Track::new("continued", "Continue target", "Artist");
    continued.duration_seconds = Some(200);
    history.record(HistoryEntry::from_track(
        &continued,
        None,
        PlaybackOutcome::Stopped,
        60,
    ));

    let mut duplicate = Track::new("duplicate", "Duplicate resume", "Artist");
    duplicate.duration_seconds = Some(300);
    history.record(HistoryEntry::from_track(
        &duplicate,
        None,
        PlaybackOutcome::Stopped,
        80,
    ));

    let mut state = AppState::new();
    assert!(
        state
            .resume_points
            .record("continued", 60.0, 200.0, chrono::Utc::now())
    );
    assert!(
        state
            .resume_points
            .record("duplicate", 80.0, 300.0, chrono::Utc::now())
    );
    state.pending_resume = Some(ytm_tui::app::state::PendingResume {
        track: duplicate,
        position_seconds: 80.0,
        armed: true,
        play_on_load: false,
    });
    state.activity.push(ActivityEvent::new(
        ActivityKind::Queued,
        "Queued truthfully",
        "Artist",
    ));
    let out = render_to_string(&mut state, Some(&history), 150, 40);
    assert!(!out.contains("CONTINUE LISTENING"), "removed:\n{out}");
    assert!(!out.contains("ACTIVITY"), "removed:\n{out}");
    assert!(
        !out.contains("Queued truthfully"),
        "activity data hidden:\n{out}"
    );
}
