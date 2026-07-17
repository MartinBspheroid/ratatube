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
fn empty_search_state_shows_prompt() {
    let mut state = AppState::new();
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
fn now_playing_bar_renders_when_track_loaded() {
    let mut state = AppState::new();
    state.current_track = Some(Track::new("abc", "Teardrop", "Massive Attack"));
    state.playback.status = ytm_tui::playback::PlaybackStatus::Playing;
    state.playback.position_seconds = 161.0;
    state.playback.duration_seconds = Some(330.0);
    state.playback.volume = 72;
    let out = render_to_string(&mut state, None, 100, 30);
    assert!(out.contains("Teardrop"), "title:\n{out}");
    assert!(out.contains("02:41 / 05:30"), "times:\n{out}");
    assert!(out.contains("72%"), "volume:\n{out}");
    assert!(out.contains("[PLAY]"), "ascii playing icon:\n{out}");
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
    assert!(out.contains("Search"), "tabs still render:\n{out}");
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
    let out = render_to_string(&mut state, Some(&history), 100, 30);
    assert!(out.contains("Song X"), "history entry:\n{out}");
    assert!(out.contains("Completed"), "outcome:\n{out}");
}

#[test]
fn visual_dump_full_screen() {
    let mut state = AppState::new();
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
    assert!(out.contains("Results (20)"));
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
    assert!(out.contains("Queue (8)"));

    let mut state = AppState::new();
    state.view = ytm_tui::app::state::View::Playlists;
    let mut p1 = ytm_tui::playlists::Playlist::new("Late Night Coding");
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
    assert!(out.contains("Playlists (2)"));
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
            total_entries: 200,
            imported: 183,
            unavailable: 5,
            duplicates: 12,
            missing_metadata: 0,
        },
        playlist: Box::new(ytm_tui::playlists::Playlist::new("Popular Music Videos")),
    });
    let out = render_to_string(&mut state, None, 100, 30);
    println!("\n{out}");
    assert!(out.contains("183"));
}
