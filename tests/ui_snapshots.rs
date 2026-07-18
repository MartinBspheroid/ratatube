//! Ratatui buffer snapshot tests (PRD section 22): empty states, search
//! results, queue, now-playing bar, ASCII mode, and narrow terminals.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use ytm_tui::app::state::AppState;
use ytm_tui::history::HistoryService;
use ytm_tui::history::model::{HistoryEntry, PlaybackOutcome};
use ytm_tui::media::Track;
use ytm_tui::media::search::SearchState;

/// Render the app to a string for snapshot-style assertions.
fn render_to_string(
    state: &mut AppState,
    history: Option<&HistoryService>,
    w: u16,
    h: u16,
) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).expect("terminal");
    ytm_tui::ui::render_with(&mut terminal, state, history).expect("render");
    buffer_to_string(terminal.backend().buffer())
}

/// Flatten a ratatui buffer to a plain string (row-major, newline-separated).
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                out.push_str(cell.symbol());
            }
        }
        out.push('\n');
    }
    out
}

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

#[test]
fn empty_search_state_shows_prompt() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(
        out.contains("Type a query and press Enter"),
        "search hint:\n{out}"
    );
    assert!(out.contains("[SEARCH] Search"), "header tabs:\n{out}");
    assert!(out.contains("mpv down"), "status:\n{out}");
}

#[test]
fn empty_queue_state() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Queue;
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Queue is empty"), "empty queue:\n{out}");
}

#[test]
fn search_results_render_sanitized() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    let mut track = Track::new("abc123", "Nice Song", "Good Artist");
    track.title = "Bad\u{1b}[2JTitle".to_string();
    state.search = SearchState::Results {
        query: "q".to_string(),
        tracks: vec![track],
    };
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Good Artist"), "artist:\n{out}");
    assert!(!out.contains('\u{1b}'), "no control chars leak:\n{out:?}");
}

#[test]
fn search_table_has_membership_icons_and_minimal_selected_metadata() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    let mut track = Track::new("selected-id", "Selected width-safe", "Selected channel");
    track.duration_seconds = Some(213);
    state.search = SearchState::Results {
        query: "selected".to_string(),
        tracks: vec![track.clone()],
    };
    let before = render_to_string(&mut state, None, 150, 40);
    for label in ["SELECTED", "Duration", "Channel"] {
        assert!(before.contains(label), "missing {label}:\n{before}");
    }
    assert!(
        before.contains("CHANNEL"),
        "search column semantics:\n{before}"
    );
    assert!(
        !before.contains("ARTIST"),
        "channel mislabeled as artist:\n{before}"
    );
    for removed in ["Source", "ID", "In queue", "+"] {
        assert!(
            !before.contains(removed),
            "removed field {removed}:\n{before}"
        );
    }
    assert!(
        !before.contains("QUICK ACTIONS"),
        "footer is the only shortcut surface:\n{before}"
    );
    for deferred in ["Published", "views", "likes", "Bitrate", "Format", "Tags"] {
        assert!(
            !before.contains(deferred),
            "deferred row {deferred}:\n{before}"
        );
    }

    state.queue.push(track.clone());
    let mut playlist = ytm_tui::playlists::Playlist::new("Membership");
    playlist
        .tracks
        .push(ytm_tui::playlists::model::PlaylistTrack::from(&track));
    state.playlists.push(playlist);
    let after = render_to_string(&mut state, None, 150, 40);
    let result_row = after
        .lines()
        .find(|line| line.contains("Selected width-safe"))
        .expect("selected result row");
    assert!(result_row.contains("[QUEUE]"), "queued icon:\n{after}");
    assert!(result_row.contains('*'), "playlist dot:\n{after}");
}

#[test]
fn narrow_search_detail_is_a_local_modal() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    state.search = SearchState::Results {
        query: "narrow".to_string(),
        tracks: vec![Track::new("narrow-id", "Narrow selected", "Channel")],
    };
    let list = render_to_string(&mut state, None, 80, 24);
    assert!(
        !list.contains("QUICK ACTIONS"),
        "detail hidden by default:\n{list}"
    );
    assert!(
        list.lines()
            .last()
            .unwrap_or_default()
            .contains("i details")
    );

    state.search_detail_open = true;
    let modal = render_to_string(&mut state, None, 80, 24);
    assert!(modal.contains("SELECTED"), "modal detail:\n{modal}");
    assert!(
        !modal.contains("QUICK ACTIONS"),
        "modal has no duplicate actions:\n{modal}"
    );
}

#[test]
fn now_playing_bar_renders_when_track_loaded() {
    let mut state = AppState::new();
    state.current_track = Some(Track::new("abc", "Teardrop", "Massive Attack"));
    state.playback.status = ytm_tui::playback::PlaybackStatus::Playing;
    state.playback.position_seconds = 161.0;
    state.playback.duration_seconds = Some(330.0);
    state.playback.volume = 72;
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Teardrop"), "title:\n{out}");
    assert!(
        out.contains("02:41 / 02:49"),
        "current/remaining times:\n{out}"
    );
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
    state.mpv_ready = true;
    state.yt_dlp_ready = true;
    state.playback.volume = 58;
    state.queue.push(Track::new("q", "Queued", "Artist"));

    let collapsed = render_to_string(&mut state, None, 99, 24);
    assert!(
        collapsed.contains("ytm v0.1.0"),
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
        state.view = ytm_tui::app::state::View::Search;
        state.search = SearchState::Results {
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
    state.view = ytm_tui::app::state::View::Search;
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
    for view in ytm_tui::app::state::View::TABS {
        let mut state = AppState::new();
        state.view = view;
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

#[test]
fn history_view_lists_entries() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::History;
    let dir = tempfile::tempdir().expect("tempdir");
    let mut history = HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
    history.record(HistoryEntry::from_track(
        &Track::new("xyz", "Song X", "Artist Y"),
        None,
        PlaybackOutcome::Completed,
        200,
    ));
    history.record(HistoryEntry::from_track(
        &Track::new("xyz", "Song X newest", "Artist Y"),
        None,
        PlaybackOutcome::Stopped,
        20,
    ));
    history.record(HistoryEntry::from_track(
        &Track::new("other", "Other song", "Other channel"),
        None,
        PlaybackOutcome::Completed,
        100,
    ));
    let out = render_to_string(&mut state, Some(&history), 100, 30);
    assert!(out.contains("HISTORY (2)"), "unique history count:\n{out}");
    assert!(
        out.contains("Song X newest"),
        "newest history entry:\n{out}"
    );
    assert!(!out.contains("Song X —"), "older duplicate hidden:\n{out}");
    assert!(out.contains("Completed"), "outcome:\n{out}");
}

#[test]
fn visual_dump_full_screen() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    state.mpv_ready = true;
    state.yt_dlp_ready = true;
    state.search = SearchState::Results {
        query: "massive attack".to_string(),
        tracks: (0..20)
            .map(|i| {
                let mut t = Track::new(
                    format!("id{i}"),
                    format!("Track Title {i}"),
                    "Massive Attack",
                );
                t.duration_seconds = Some(213 + i * 17);
                t
            })
            .collect(),
    };
    state.selected_index = 4;
    state.current_track = Some(Track::new("u7K72X4eo_s", "Teardrop", "Massive Attack"));
    state.playback.status = ytm_tui::playback::PlaybackStatus::Playing;
    state.playback.position_seconds = 161.0;
    state.playback.duration_seconds = Some(330.0);
    state.playback.volume = 72;
    state.queue.push(Track::new("a", "Angel", "Massive Attack"));
    state
        .queue
        .push(Track::new("b", "Teardrop", "Massive Attack"));
    state.queue.position = Some(1);

    let out = render_to_string(&mut state, None, 110, 34);
    println!("\n{out}");
    assert!(out.contains("20 results for \"massive attack\""));
}

#[test]
fn visual_dump_queue_and_playlists() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Queue;
    state.queue.shuffle = true;
    for i in 0..8 {
        state.queue.push(Track::new(
            format!("id{i}"),
            format!("Queue Song {i}"),
            "Artist",
        ));
    }
    state.queue.position = Some(2);
    state.selected_index = 2;
    let out = render_to_string(&mut state, None, 100, 24);
    println!("\n{out}");
    assert!(out.contains("QUEUE (8)"));

    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Playlists;
    let mut p1 = ytm_tui::playlists::Playlist::new("Late Night Coding");
    p1.description = "Focus music for long sessions".to_string();
    p1.tracks = (0..20)
        .map(|i| {
            ytm_tui::playlists::model::PlaylistTrack::from(&Track::new(
                format!("t{i}"),
                format!("Song {i}"),
                "Artist",
            ))
        })
        .collect();
    let p2 = ytm_tui::playlists::Playlist::new("Workout");
    state.playlists = vec![p1, p2];
    let out = render_to_string(&mut state, None, 100, 24);
    println!("\n{out}");
    assert!(out.contains("PLAYLISTS (2)"));
    assert!(out.contains("PLAYLIST INFO"), "selected metadata:\n{out}");
    assert!(out.contains("TRACK PREVIEW (20)"), "track table:\n{out}");
    assert!(out.contains("CHANNEL"), "playlist column semantics:\n{out}");
    assert!(
        !out.contains("ARTIST"),
        "channel mislabeled as artist:\n{out}"
    );
    let first_track = out
        .lines()
        .find(|line| line.contains("Song 0"))
        .expect("first playlist track");
    assert!(
        first_track.find("Song 0") < first_track.find("Artist"),
        "video title must precede channel:\n{out}"
    );
    assert!(
        !out.contains("[⏎ Open]") && !out.contains("[p Play]"),
        "panel must not duplicate footer shortcuts:\n{out}"
    );
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
    assert!(out.contains("TRACKLIST (2/3)"), "chapter counter:\n{out}");
    assert!(out.contains("Second Tune"), "chapter titles:\n{out}");
    assert!(out.contains("UP NEXT"), "up next pane:\n{out}");
    assert!(out.contains("Next Song"), "upcoming track:\n{out}");
    assert!(!out.contains("Shuffle:"), "mode row removed:\n{out}");
    assert!(!out.contains("vol 0%"), "volume row removed:\n{out}");
    // Playback information appears once, in the shared bottom bar.
    assert_eq!(
        out.matches("04:00 / 1:54:35").count(),
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
    assert!(ultra.contains("QUEUE · H/L FOCUS"), "ultra queue:\n{ultra}");
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

#[test]
fn history_top_mode_shows_play_counts() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::History;
    state.history_view_mode = ytm_tui::app::state::HistoryViewMode::Top;
    let dir = tempfile::tempdir().expect("tempdir");
    let mut history = HistoryService::load(&dir.path().join("h.json"), 500).expect("load");
    for _ in 0..3 {
        history.record(HistoryEntry::from_track(
            &Track::new("a", "Favourite Mix", "DJ A"),
            None,
            PlaybackOutcome::Completed,
            3600,
        ));
    }
    history.record(HistoryEntry::from_track(
        &Track::new("b", "One-off", "DJ B"),
        None,
        PlaybackOutcome::Skipped,
        60,
    ));
    let out = render_to_string(&mut state, Some(&history), 110, 30);
    assert!(out.contains("3 plays · 3 tries"), "play count:\n{out}");
    assert!(out.contains("Favourite Mix"), "aggregated title:\n{out}");
    assert!(out.contains("3:00:00 total"), "total listened:\n{out}");
    assert!(out.contains("· TOP"), "mode label:\n{out}");
}

#[test]
fn queue_filter_bar_narrows_list() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Queue;
    state.queue.push(Track::new("a", "ISS006", "Skee Mask"));
    state.queue.push(Track::new("b", "Xtal", "Aphex Twin"));
    state.list_filter = Some("skee".to_string());
    state.visible_indices = Some(vec![0]);
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("/skee"), "filter bar:\n{out}");
    assert!(out.contains("1 of 2"), "match count:\n{out}");
    assert!(out.contains("ISS006"), "matching row:\n{out}");
    assert!(!out.contains("Xtal"), "filtered-out row:\n{out}");
}

#[test]
fn playlist_picker_modal_renders() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    state.playlists = vec![
        ytm_tui::playlists::Playlist::new("Techno Sets"),
        ytm_tui::playlists::Playlist::new("Ambient"),
    ];
    state.picker = Some(ytm_tui::app::state::PickerState {
        track: Track::new("t", "Song", "Artist"),
        filter: "tech".to_string(),
        selected: 0,
    });
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Add to playlist"), "modal title:\n{out}");
    assert!(
        out.contains("New playlist \"tech\""),
        "create entry:\n{out}"
    );
    assert!(out.contains("Techno Sets"), "matching playlist:\n{out}");
    assert!(
        !out.contains("Ambient  (0)"),
        "non-matching playlist hidden:\n{out}"
    );
}

#[test]
fn playlist_json_prompt_explains_paste_and_submit() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Playlists;
    state.prompt = Some(ytm_tui::app::state::PromptState {
        purpose: ytm_tui::app::state::PromptPurpose::ImportPlaylistJson,
        buffer: "{\n  \"version\": 1\n}".to_string(),
    });

    let out = render_to_string(&mut state, None, 100, 30);

    assert!(out.contains("Paste playlist JSON"), "modal title:\n{out}");
    assert!(out.contains("Enter import"), "submit instruction:\n{out}");
    assert!(out.contains("version"), "pasted content preview:\n{out}");
}

#[test]
fn playlist_editor_uses_stateful_table_inspector_and_edit_popup() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::PlaylistDetail;
    let mut playlist = ytm_tui::playlists::Playlist::new("Editable");
    playlist.description = "A useful description".to_string();
    playlist
        .tracks
        .push(ytm_tui::playlists::model::PlaylistTrack::from(&Track::new(
            "one",
            "First video",
            "First channel",
        )));
    playlist
        .tracks
        .push(ytm_tui::playlists::model::PlaylistTrack::from(&Track::new(
            "two",
            "Selected video",
            "Selected channel",
        )));
    state.playlists.push(playlist);
    state.selected_playlist = Some(0);
    state.selected_index = 1;

    let browse = render_to_string(&mut state, None, 120, 32);
    assert!(
        browse.contains("PLAYLIST EDITOR"),
        "editor title:\n{browse}"
    );
    assert!(browse.contains("SELECTED TRACK"), "inspector:\n{browse}");
    assert!(browse.contains("Selected video"), "selected row:\n{browse}");
    assert!(browse.contains("BROWSE MODE"), "mode status:\n{browse}");

    state.playlist_editor = Some(ytm_tui::app::state::PlaylistEditorState {
        name: "Edited name".to_string(),
        description: "Edited description".to_string(),
        field: ytm_tui::app::state::PlaylistEditorField::Name,
    });
    let editing = render_to_string(&mut state, None, 120, 32);
    assert!(editing.contains("Edit playlist"), "popup title:\n{editing}");
    assert!(editing.contains("Edited name"), "name field:\n{editing}");
    assert!(
        editing.contains("Edited description"),
        "description field:\n{editing}"
    );
    assert!(
        editing.contains("Tab next field"),
        "field instructions:\n{editing}"
    );
}

#[test]
fn visual_dump_help_and_modal() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Help;
    let out = render_to_string(&mut state, None, 100, 34);
    println!("\n{out}");
    assert!(out.contains("Seek 5 seconds"));

    let mut state = AppState::new();
    state.import = Some(ytm_tui::app::state::ImportState::Review {
        summary: ytm_tui::playlists::import::ImportSummary {
            remote_title: "Popular Music Videos".to_string(),
            remote_url: "https://...".to_string(),
            total_entries: 205,
            imported: 183,
            deleted: 2,
            private: 3,
            unavailable: 5,
            duplicates: 12,
            missing_id: 0,
            missing_title: 0,
        },
        playlist: Box::new(ytm_tui::playlists::Playlist::new("Popular Music Videos")),
    });
    let out = render_to_string(&mut state, None, 100, 30);
    println!("\n{out}");
    assert!(out.contains("183"));
}

#[test]
fn help_is_scrollable_at_minimum_supported_viewport() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Help;
    let first = render_to_string(&mut state, None, 80, 24);
    assert!(first.contains("J/K OR PGUP/PGDN SCROLL"));
    state.help_scroll = u16::MAX;
    let last = render_to_string(&mut state, None, 80, 24);
    assert!(last.contains("Message log"), "last help page:\n{last}");
    assert!(last.contains("Quit"), "last help page:\n{last}");
}

#[test]
fn renderer_exposes_exact_search_result_hit_rows() {
    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Search;
    state.search = SearchState::Results {
        query: "query".to_string(),
        tracks: vec![Track::new("id", "title", "artist")],
    };
    let _ = render_to_string(&mut state, None, 80, 24);
    assert!(state.list_hit_area.y > state.main_area.y + 3);
    assert!(state.list_hit_area.height > 0);
}
