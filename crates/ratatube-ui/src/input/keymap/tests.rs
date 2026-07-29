use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatube_domain::action::UiMsg;

use super::{HELP_SECTIONS, playing_pane_action, route, view_action};
use crate::render::layout::Breakpoint;
use crate::state::{Focus, PlayingPane, View};
use ratatube_domain::action::{
    Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction,
};

#[test]
fn home_new_playlist_hint_has_a_working_binding() {
    let key = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::NONE);
    assert!(matches!(
        route(&key, Focus::Content, View::Home),
        Some(Action::Playlists(PlaylistAction::OpenPrompt(
            crate::state::PromptPurpose::NewPlaylist
        )))
    ));
}

#[test]
fn playlist_action_row_matches_real_bindings() {
    let action = |code| view_action(&KeyEvent::new(code, KeyModifiers::NONE), View::Playlists);
    assert!(matches!(
        action(KeyCode::Enter),
        Some(Action::Playlists(PlaylistAction::OpenPlaylistDetail))
    ));
    for (key, purpose) in [
        ('N', crate::state::PromptPurpose::NewPlaylist),
        ('i', crate::state::PromptPurpose::ImportPlaylistUrl),
        ('I', crate::state::PromptPurpose::ImportPlaylistJson),
        ('R', crate::state::PromptPurpose::RenamePlaylist),
    ] {
        assert!(matches!(
            action(KeyCode::Char(key)),
            Some(Action::Playlists(PlaylistAction::OpenPrompt(actual))) if actual == purpose
        ));
    }
    assert!(matches!(
        action(KeyCode::Char('x')),
        Some(Action::Playlists(PlaylistAction::DeleteSelectedPlaylist))
    ));
    assert!(matches!(
        view_action(
            &KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
            View::PlaylistDetail
        ),
        Some(Action::Playlists(PlaylistAction::OpenPlaylistEditor))
    ));
}

#[test]
fn help_close_and_scroll_bindings_take_precedence_over_globals() {
    let close = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
    let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert!(matches!(
        route(&close, Focus::Content, View::Help),
        Some(Action::Ui(UiMsg::CloseHelp))
    ));
    assert!(matches!(
        route(&down, Focus::Content, View::Help),
        Some(Action::Ui(UiMsg::ScrollHelp(1)))
    ));
}

#[test]
fn ultra_wide_queue_pane_owns_navigation_and_play() {
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    assert!(matches!(
        playing_pane_action(
            &key(KeyCode::Char('l')),
            PlayingPane::Info,
            Breakpoint::UltraWide
        ),
        Some(Action::Ui(UiMsg::CyclePlayingPane))
    ));
    assert!(matches!(
        playing_pane_action(
            &key(KeyCode::Down),
            PlayingPane::Queue,
            Breakpoint::UltraWide
        ),
        Some(Action::Ui(UiMsg::SelectNext))
    ));
    assert!(matches!(
        playing_pane_action(
            &key(KeyCode::Enter),
            PlayingPane::Queue,
            Breakpoint::UltraWide
        ),
        Some(Action::Playback(PlaybackAction::PlaySelected))
    ));
    assert!(
        playing_pane_action(
            &key(KeyCode::Char('l')),
            PlayingPane::Info,
            Breakpoint::Wide
        )
        .is_none()
    );
}

#[test]
fn search_detail_and_browser_keys_dispatch_real_actions() {
    let key = |character| KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
    assert!(matches!(
        route(&key('i'), Focus::Content, View::Search),
        Some(Action::Ui(UiMsg::ToggleSearchDetail))
    ));
    for view in [View::Search, View::NowPlaying] {
        assert!(matches!(
            route(&key('o'), Focus::Content, view),
            Some(Action::Navigation(NavigationAction::OpenInBrowser))
        ));
    }
}

#[test]
fn uppercase_c_clears_queue_and_history_while_lowercase_opens_track_actions() {
    let key = |character| KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
    assert!(matches!(
        route(&key('c'), Focus::Content, View::Queue),
        Some(Action::Navigation(NavigationAction::OpenTrackContext))
    ));
    assert!(matches!(
        route(&key('C'), Focus::Content, View::Queue),
        Some(Action::Queue(QueueAction::ClearQueue))
    ));
    assert!(matches!(
        route(&key('c'), Focus::Content, View::History),
        Some(Action::Navigation(NavigationAction::OpenTrackContext))
    ));
    assert!(matches!(
        route(&key('C'), Focus::Content, View::History),
        Some(Action::History(HistoryAction::ClearHistory))
    ));
}

#[test]
fn help_documents_both_uppercase_clear_shortcuts() {
    let documented = HELP_SECTIONS
        .iter()
        .flat_map(|(_, entries)| entries.iter())
        .collect::<Vec<_>>();
    assert!(documented.contains(&&("C", "Clear queue (asks first)")));
    assert!(documented.contains(&&("C", "Clear history (asks first)")));
}
