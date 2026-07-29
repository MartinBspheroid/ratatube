use crate::app::action::{ExternalCommandKind, ExternalCommandTarget};
use crate::app::state::View;
use crate::app::tests::test_app;
use crate::app::track_context::TrackContextAction;
use crate::media::search::SearchState;

use super::dispatch::open_menu_for_action;
use super::track;

fn app_with_open_browser_menu() -> crate::app::App {
    let (_temp, mut app) = test_app();
    let selected = track("selected", "Selected track");
    app.state.ui.view = View::Search;
    app.state.domain.search = SearchState::Results {
        query: "selected".to_string(),
        tracks: vec![selected],
    };
    open_menu_for_action(&mut app, TrackContextAction::OpenInBrowser);
    app
}

#[test]
fn every_external_command_failure_keeps_the_track_menu_open() {
    for failure in [
        "spawn failed",
        "command timed out",
        "command exited with status 3",
    ] {
        let mut app = app_with_open_browser_menu();

        app.finish_external_command(
            ExternalCommandKind::Browser,
            ExternalCommandTarget::TrackContext {
                track_id: "selected".to_string(),
                generation: app.state.ui.track_context_generation,
            },
            Err(failure.to_string()),
        );

        assert!(
            app.state.ui.track_context_menu.is_some(),
            "failure: {failure}"
        );
        assert!(
            app.state
                .ui
                .notification
                .as_ref()
                .is_some_and(|notice| notice.is_error && notice.message.contains(failure))
        );
    }
}

#[test]
fn zero_exit_completion_closes_only_the_matching_track_menu() {
    let mut app = app_with_open_browser_menu();
    let generation = app.state.ui.track_context_generation;

    app.finish_external_command(
        ExternalCommandKind::Browser,
        ExternalCommandTarget::TrackContext {
            track_id: "selected".to_string(),
            generation,
        },
        Ok(()),
    );

    assert!(app.state.ui.track_context_menu.is_none());
    assert!(
        app.state
            .ui
            .notification
            .as_ref()
            .is_some_and(|notice| !notice.is_error)
    );
}

#[test]
fn stale_success_does_not_close_reopened_menu_for_the_same_track() {
    let mut app = app_with_open_browser_menu();
    let stale_generation = app.state.ui.track_context_generation;
    app.state.ui.track_context_menu = None;
    crate::app::track_context::open_track_context(&mut app.state, app.history.as_ref());

    app.finish_external_command(
        ExternalCommandKind::Browser,
        ExternalCommandTarget::TrackContext {
            track_id: "selected".to_string(),
            generation: stale_generation,
        },
        Ok(()),
    );

    assert!(app.state.ui.track_context_menu.is_some());
    assert_ne!(app.state.ui.track_context_generation, stale_generation);
}
