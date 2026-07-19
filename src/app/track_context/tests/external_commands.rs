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
    app.state.view = View::Search;
    app.state.search = SearchState::Results {
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
            },
            Err(failure.to_string()),
        );

        assert!(app.state.track_context_menu.is_some(), "failure: {failure}");
        assert!(
            app.state
                .notification
                .as_ref()
                .is_some_and(|notice| notice.is_error && notice.message.contains(failure))
        );
    }
}

#[test]
fn zero_exit_completion_closes_only_the_matching_track_menu() {
    let mut app = app_with_open_browser_menu();

    app.finish_external_command(
        ExternalCommandKind::Browser,
        ExternalCommandTarget::TrackContext {
            track_id: "selected".to_string(),
        },
        Ok(()),
    );

    assert!(app.state.track_context_menu.is_none());
    assert!(
        app.state
            .notification
            .as_ref()
            .is_some_and(|notice| !notice.is_error)
    );
}
