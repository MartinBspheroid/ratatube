use super::*;

#[test]
fn history_view_lists_entries() {
    let mut state = AppState::new();
    state.ui.view = ratatube::app::state::View::History;
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
    state.ui.view = ratatube::app::state::View::Search;
    state.domain.mpv_ready = true;
    state.domain.yt_dlp_ready = true;
    state.domain.search = SearchState::Results {
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
    state.ui.selected_index = 4;
    state.domain.current_track = Some(Track::new("u7K72X4eo_s", "Teardrop", "Massive Attack"));
    state.domain.playback.status = ratatube::playback::PlaybackStatus::Playing;
    state.domain.playback.position_seconds = 161.0;
    state.domain.playback.duration_seconds = Some(330.0);
    state.domain.playback.volume = 72;
    state
        .domain
        .queue
        .push(Track::new("a", "Angel", "Massive Attack"));
    state
        .domain
        .queue
        .push(Track::new("b", "Teardrop", "Massive Attack"));
    state.domain.queue.position = Some(1);

    let out = render_to_string(&mut state, None, 110, 34);
    println!("\n{out}");
    assert!(out.contains("20 results for \"massive attack\""));
}

#[test]
fn visual_dump_queue_and_playlists() {
    let mut state = AppState::new();
    state.ui.view = ratatube::app::state::View::Queue;
    state.domain.queue.shuffle = true;
    for i in 0..8 {
        state.domain.queue.push(Track::new(
            format!("id{i}"),
            format!("Queue Song {i}"),
            "Artist",
        ));
    }
    state.domain.queue.position = Some(2);
    state.ui.selected_index = 2;
    let out = render_to_string(&mut state, None, 100, 24);
    println!("\n{out}");
    assert!(out.contains("QUEUE (8)"));

    let mut state = AppState::new();
    state.ui.view = ratatube::app::state::View::Playlists;
    let mut p1 = ratatube::playlists::Playlist::new("Late Night Coding");
    p1.description = "Focus music for long sessions".to_string();
    p1.tracks = (0..20)
        .map(|i| {
            ratatube::playlists::model::PlaylistTrack::from(&Track::new(
                format!("t{i}"),
                format!("Song {i}"),
                "Artist",
            ))
        })
        .collect();
    let p2 = ratatube::playlists::Playlist::new("Workout");
    state.domain.playlists = vec![p1, p2];
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
