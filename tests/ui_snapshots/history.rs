use super::*;

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
    assert!(
        out.contains("finished"),
        "human outcome wording (not Debug):\n{out}"
    );
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
