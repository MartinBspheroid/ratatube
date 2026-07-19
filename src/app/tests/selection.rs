use super::*;

#[tokio::test]
async fn search_queue_action_updates_selected_panel_membership_source() {
    let (_temp, mut app) = test_app();
    let track = Track::new("selected", "Selected", "Channel");
    app.state.view = View::Search;
    app.state.search = crate::media::search::SearchState::Results {
        query: "selected".to_string(),
        tracks: vec![track],
    };
    let (action_tx, mut action_rx) = mpsc::channel(4);
    app.handle_action(Action::Queue(QueueAction::AddSelectedToQueue), &action_tx)
        .await;
    let dispatched = action_rx.try_recv().expect("resolved queue action");
    assert!(
        matches!(&dispatched, Action::Queue(QueueAction::AddToQueue(track)) if track.id == "selected")
    );
    app.handle_action(dispatched, &action_tx).await;
    assert!(
        app.state
            .queue
            .tracks
            .iter()
            .any(|track| track.id == "selected")
    );
}

#[test]
fn unchanged_filter_reuses_derived_indices_until_list_mutation() {
    let (_temp, mut app) = test_app();
    app.state.view = View::Queue;
    app.state.queue.push(Track::new("a", "Alpha", "Artist"));
    app.state.queue.push(Track::new("b", "Beta", "Artist"));
    app.state.list_filter = Some("alpha".to_string());
    app.sync_list_view();
    let first = app
        .state
        .visible_indices
        .as_ref()
        .expect("filtered indices")
        .as_ptr();
    app.sync_list_view();
    assert_eq!(
        first,
        app.state
            .visible_indices
            .as_ref()
            .expect("cached indices")
            .as_ptr()
    );

    app.list_revision += 1;
    app.sync_list_view();
    assert_eq!(app.state.visible_indices.as_deref(), Some([0].as_slice()));
}

#[test]
fn recent_history_selection_maps_through_newest_unique_entries() {
    let (_temp, mut app) = test_app();
    let history = app.history.as_mut().expect("history service");
    for track in [
        Track::new("a", "Old A", "Channel"),
        Track::new("b", "Only B", "Channel"),
        Track::new("a", "Newest A", "Channel"),
    ] {
        history.record(crate::history::model::HistoryEntry::from_track(
            &track,
            None,
            PlaybackOutcome::Stopped,
            1,
        ));
    }
    app.state.view = View::History;
    app.sync_list_view();
    assert_eq!(app.state.history_len, 2);
    assert_eq!(
        app.state.visible_indices.as_deref(),
        Some([0, 1].as_slice())
    );

    app.state.list_filter = Some("Only B".to_string());
    app.sync_list_view();

    assert_eq!(app.state.visible_indices.as_deref(), Some([1].as_slice()));
    assert_eq!(
        app.resolve_selected_track().map(|track| track.id),
        Some("b".to_string())
    );
}
