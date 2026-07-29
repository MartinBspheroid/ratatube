use super::*;
use crate::app::actions::UiMsg;

#[test]
fn quit_persists_and_exits() {
    let mut state = AppState::new();
    let effects = reduce(&mut state, Action::Navigation(NavigationAction::Quit));
    assert!(!state.ui.running);
    assert!(effects.contains(&Effect::PersistQueue));
    assert!(effects.contains(&Effect::Exit));
}

#[test]
fn details_failure_replaces_loading_for_current_track() {
    let mut state = AppState::new();
    state.domain.current_track = Some(track("current"));
    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Details);
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::DetailsStarted {
            operation_id: ticket.id(),
            track_id: "current".to_string(),
        }),
    );
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::DetailsFailed {
            operation_id: ticket.id(),
            track_id: "current".to_string(),
            message: "offline".to_string(),
        }),
    );

    assert!(matches!(
        state.domain.details_status,
        DetailsStatus::Failed { ref message, .. } if message == "offline"
    ));
}

#[test]
fn play_track_waits_for_resolution_before_replacing_current_track() {
    let mut state = AppState::new();
    let effects = reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlayTrack(track("a"))),
    );
    assert_eq!(state.domain.queue.tracks.len(), 1);
    assert!(state.domain.current_track.is_none());
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::ResolveAndPlay { .. }))
    );

    let mut operations = OperationRegistry::default();
    let ticket = operations.start(OperationKind::Playback);
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackResolveStarted {
            operation_id: ticket.id(),
            queue_position: 0,
            track_id: "a".to_string(),
        }),
    );
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackResolved {
            operation_id: ticket.id(),
            queue_position: 0,
            track_id: "a".to_string(),
            url: "https://stream.invalid/a".to_string(),
        }),
    );
    assert_eq!(
        state.domain.current_track.as_ref().map(|t| t.id.as_str()),
        Some("a")
    );
}
#[test]
fn superseded_playback_completion_cannot_replace_current_track() {
    let mut state = AppState::new();
    state.domain.queue.push(track("requested"));
    state.domain.queue.position = Some(0);
    let mut operations = OperationRegistry::default();
    let stale = operations.start(OperationKind::Playback);
    let current = operations.start(OperationKind::Playback);
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackResolveStarted {
            operation_id: current.id(),
            queue_position: 0,
            track_id: "requested".to_string(),
        }),
    );
    reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackResolved {
            operation_id: stale.id(),
            queue_position: 0,
            track_id: "requested".to_string(),
            url: "https://stream.invalid/stale".to_string(),
        }),
    );

    assert!(state.domain.current_track.is_none());
    assert!(matches!(
        state.domain.playback_resolution,
        OperationStatus::Loading { operation_id } if operation_id == current.id()
    ));
}

#[test]
fn move_selected_reorders_and_follows() {
    let mut state = AppState::new();
    state.ui.view = View::Queue;
    state.domain.queue.push(track("a"));
    state.domain.queue.push(track("b"));
    state.domain.queue.push(track("c"));
    state.domain.queue.position = Some(0);
    state.ui.selected_index = 0;
    let effects = reduce(
        &mut state,
        Action::Queue(QueueAction::MoveSelectedInQueue(1)),
    );
    assert_eq!(state.domain.queue.order, vec![1, 0, 2]);
    assert_eq!(state.ui.selected_index, 1);
    assert_eq!(state.domain.queue.position, Some(1));
    assert!(effects.contains(&Effect::PersistQueue));
    // Moving past the end is a no-op.
    state.ui.selected_index = 2;
    let effects = reduce(
        &mut state,
        Action::Queue(QueueAction::MoveSelectedInQueue(1)),
    );
    assert!(effects.is_empty());
}

#[test]
fn ultra_wide_playing_queue_focus_selects_without_duplicating() {
    let mut state = AppState::new();
    state.ui.view = View::NowPlaying;
    state.ui.screen_area = ratatui::layout::Rect::new(0, 0, 180, 48);
    state.domain.queue.push(track("a"));
    state.domain.queue.push(track("b"));
    state.domain.queue.position = Some(0);

    assert!(reduce(&mut state, Action::Ui(UiMsg::CyclePlayingPane)).is_empty());
    assert_eq!(state.ui.playing_pane, PlayingPane::Queue);
    assert!(reduce(&mut state, Action::Ui(UiMsg::SelectNext)).is_empty());
    assert_eq!(state.ui.selected_index, 1);
    let effects = reduce(&mut state, Action::Playback(PlaybackAction::PlaySelected));

    assert_eq!(
        state.domain.queue.tracks.len(),
        2,
        "selection must not duplicate"
    );
    assert_eq!(state.domain.queue.position, Some(1));
    assert_eq!(
        effects,
        vec![
            Effect::ResolveAndPlay {
                track_index_in_queue: 1
            },
            Effect::PersistQueue
        ]
    );
}

#[test]
fn channel_selection_queues_and_plays_exact_track() {
    let mut state = AppState::new();
    state.ui.view = View::Channel;
    state.ui.selected_index = 1;
    state.domain.channel = Some(crate::app::state::ChannelState {
        name: "Channel".into(),
        url: "https://www.youtube.com/channel/UC1/videos".into(),
        tracks: vec![track("first"), track("selected")],
        next_page: 1,
        exhausted: true,
        loading: false,
        error: None,
        return_to: crate::app::state::ChannelNavigationSnapshot {
            view: View::Search,
            focus: crate::app::state::Focus::Content,
            selected_index: 0,
        },
        previous: None,
    });

    let effects = reduce(&mut state, Action::Playback(PlaybackAction::PlaySelected));

    assert_eq!(state.domain.queue.tracks.len(), 1);
    assert_eq!(state.domain.queue.tracks[0].id, "selected");
    assert_eq!(state.domain.queue.position, Some(0));
    assert_eq!(
        effects,
        vec![
            Effect::ResolveAndPlay {
                track_index_in_queue: 0
            },
            Effect::PersistQueue
        ]
    );
}

#[test]
fn queue_and_playback_actions_emit_only_truthful_activity_kinds() {
    use crate::history::activity::ActivityKind;

    let mut state = AppState::new();
    let effects = reduce(
        &mut state,
        Action::Queue(QueueAction::AddToQueue(track("queued"))),
    );
    assert!(effects.contains(&Effect::PersistSession));
    assert_eq!(
        state
            .domain
            .activity
            .entries()
            .front()
            .map(|event| event.kind),
        Some(ActivityKind::Queued)
    );

    state.domain.current_track = Some(track("played"));
    let effects = reduce(
        &mut state,
        Action::Playback(PlaybackAction::PlaybackEvent(PlaybackEvent::Started)),
    );
    assert!(effects.contains(&Effect::PersistSession));
    assert_eq!(
        state
            .domain
            .activity
            .entries()
            .front()
            .map(|event| event.kind),
        Some(ActivityKind::Played)
    );

    assert_eq!(
        reduce(&mut state, Action::History(HistoryAction::ClearActivity)),
        vec![Effect::PersistSession]
    );
    assert!(state.domain.activity.is_empty());
}
