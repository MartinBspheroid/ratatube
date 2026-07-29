use super::*;

#[test]
fn app_capture_resume_point_honors_boundaries() {
    let (_temp, mut app) = test_app();
    app.state.domain.current_track = Some(Track::new("track", "Track", "Artist"));
    app.state.domain.playback.duration_seconds = Some(100.0);

    app.state.domain.playback.position_seconds = 10.0;
    app.capture_resume_point();
    app.state.domain.playback.position_seconds = 95.0;
    app.capture_resume_point();
    assert_eq!(app.state.domain.resume_points.len(), 0);

    app.state.domain.playback.position_seconds = 30.0;
    app.capture_resume_point();
    assert_eq!(app.state.domain.resume_points.len(), 1);
}

#[tokio::test]
async fn pause_stop_and_track_change_capture_resume_points() {
    let (_temp, mut app) = test_app();
    let (action_tx, _action_rx) = mpsc::channel(2);
    let set_track = |app: &mut App, id: &str| {
        app.state.domain.current_track = Some(Track::new(id, id, "Artist"));
        app.state.domain.playback.position_seconds = 30.0;
        app.state.domain.playback.duration_seconds = Some(100.0);
    };

    set_track(&mut app, "pause");
    app.on_playback_event(&PlaybackEvent::PauseChanged(true));
    set_track(&mut app, "stop");
    app.handle_action(Action::Playback(PlaybackAction::Stop), &action_tx)
        .await;
    set_track(&mut app, "change");
    app.handle_action(
        Action::Playback(PlaybackAction::PlayTrack(Track::new(
            "next", "Next", "Artist",
        ))),
        &action_tx,
    )
    .await;

    let ids = app
        .state
        .domain
        .resume_points
        .entries()
        .iter()
        .map(|point| point.video_id.as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"pause"));
    assert!(ids.contains(&"stop"));
    assert!(ids.contains(&"change"));
}

#[tokio::test]
async fn session_panels_restore_even_when_playback_resume_is_offline() {
    use crate::history::activity::{ActivityEvent, ActivityKind};

    let (_temp, mut app) = test_app();
    let mut document = crate::persistence::session::SessionDocument::new(None, 0.0, 50);
    document.activity.push(ActivityEvent::new(
        ActivityKind::Queued,
        "Persisted event",
        "detail",
    ));
    assert!(
        document
            .resume_points
            .record("persisted-track", 30.0, 100.0, chrono::Utc::now())
    );
    crate::persistence::session::save(&app.paths.session_file(), &document).expect("save session");
    app.playback = None;
    let (action_tx, _action_rx) = mpsc::channel(2);

    app.init_session(&action_tx).await;

    assert_eq!(app.state.domain.activity.len(), 1);
    assert_eq!(app.state.domain.resume_points.len(), 1);
    assert!(app.state.domain.pending_resume.is_none());
}
