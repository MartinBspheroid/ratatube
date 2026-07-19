use super::*;

fn track(id: &str) -> Track {
    Track::new(id, id, "artist")
}

#[test]
fn deduplicate_keeps_first_occurrence() {
    let tracks = vec![track("a"), track("b"), track("a"), track("c"), track("b")];
    let (kept, dupes) = deduplicate(tracks);
    let ids: Vec<&str> = kept.iter().map(|track| track.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
    assert_eq!(dupes, 2);
}

#[test]
fn build_import_records_source_and_summary() {
    let (playlist, summary) = build_import(
        "My Mix".to_string(),
        "https://www.youtube.com/playlist?list=PLx".to_string(),
        Some("PLx".to_string()),
        vec![track("a"), track("a"), track("b")],
        crate::media::yt_dlp::ImportRejections {
            malformed: 0,
            missing_id: 1,
            missing_title: 1,
            deleted: 2,
            private: 2,
            unavailable: 1,
        },
    );
    assert_eq!(summary.imported, 2);
    assert_eq!(summary.duplicates, 1);
    assert_eq!(summary.deleted, 2);
    assert_eq!(summary.private, 2);
    assert_eq!(summary.unavailable, 1);
    assert_eq!(summary.missing_id, 1);
    assert_eq!(summary.missing_title, 1);
    assert!(playlist.source.is_some());
    assert_eq!(playlist.tracks.len(), 2);
}

#[test]
fn pasted_json_builds_multiple_local_playlists() {
    let json = r#"{
      "version": 1,
      "playlists": [
        {"name": "Neon Pressure", "description": "Jungle heat", "tracks": [
          {"title": "Reset", "channel": "Visages", "url": "https://music.youtube.com/watch?v=sEltKu3XP6I"}
        ]},
        {"name": "Subterranean", "tracks": [
          {"title": "Deepsoft", "channel": "SCIENIDE 1995", "url": "https://www.youtube.com/watch?v=OX838AIRC8M"}
        ]}
      ]
    }"#;
    let playlists = parse_pasted_json(json).expect("valid import");
    assert_eq!(playlists.len(), 2);
    assert_eq!(playlists[0].name, "Neon Pressure");
    assert_eq!(playlists[0].description, "Jungle heat");
    assert_eq!(playlists[0].tracks[0].id, "sEltKu3XP6I");
    assert_eq!(playlists[0].tracks[0].artist, "Visages");
    assert_eq!(playlists[1].tracks[0].id, "OX838AIRC8M");
    assert!(playlists.iter().all(|playlist| playlist.source.is_none()));
}

#[test]
fn pasted_json_reports_the_invalid_track_location() {
    let json = r#"{
      "version": 1,
      "playlists": [{"name": "Broken", "tracks": [
        {"title": "No link", "channel": "Unknown", "url": ""}
      ]}]
    }"#;
    let error = parse_pasted_json(json).expect_err("missing URL must fail");
    assert!(error.contains("Broken"), "{error}");
    assert!(error.contains("No link"), "{error}");
    assert!(error.contains("YouTube video URL"), "{error}");
}

#[test]
fn checked_in_playlist_json_contains_all_specified_tracks() {
    let playlists = parse_pasted_json(include_str!("../../../playlist.json"))
        .expect("checked-in playlist.json must remain importable");
    assert_eq!(playlists.len(), 7);
    assert_eq!(
        playlists
            .iter()
            .map(|playlist| playlist.tracks.len())
            .sum::<usize>(),
        154
    );
}
