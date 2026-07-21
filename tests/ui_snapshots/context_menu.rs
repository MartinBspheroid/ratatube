use ratatui::style::Color;

use super::*;
use ytm_tui::app::state::{TrackContextMenuState, TrackDetailsModalState, View};
use ytm_tui::app::track_context::resolve_track_context;
use ytm_tui::playlists::Playlist;
use ytm_tui::playlists::model::PlaylistTrack;

fn render_menu(state: &mut AppState) -> (ratatui::buffer::Buffer, String) {
    let context = resolve_track_context(state, None).expect("track context");
    state.ui.track_context_menu = Some(TrackContextMenuState {
        context,
        selected: 0,
    });
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    ytm_tui::ui::render_with(&mut terminal, state, None).expect("render");
    let buffer = terminal.backend().buffer().clone();
    let text = buffer_to_string(&buffer);
    (buffer, text)
}

fn assert_order(text: &str, labels: &[&str]) {
    let mut previous = 0;
    for label in labels {
        let offset = text[previous..]
            .find(label)
            .unwrap_or_else(|| panic!("missing {label:?}:\n{text}"));
        previous += offset + label.len();
    }
}

#[test]
fn track_context_menu_snapshots_exact_order_and_queue_warning_style() {
    let mut state = AppState::new();
    state.ui.view = View::Queue;
    state
        .domain
        .queue
        .push(Track::new("queue", "Queue\nTrack\u{1b}[2J", "Artist"));

    let (buffer, text) = render_menu(&mut state);

    assert_order(
        &text,
        &[
            "Play now",
            "Play next",
            "Add to playlist",
            "Visit channel",
            "Show details",
            "Open in browser",
            "Copy URL",
            "! Remove from queue",
        ],
    );
    assert!(!text.contains('\u{1b}'), "title must be sanitized:\n{text}");
    let (row, line) = text
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("! Remove from queue"))
        .expect("warning row");
    let column = line.find('!').expect("warning marker") as u16;
    let cell = buffer.cell((column, row as u16)).expect("warning cell");
    assert_eq!(cell.fg, Color::Yellow);
}

#[test]
fn track_context_menu_includes_conditional_playlist_removal_last() {
    let mut state = AppState::new();
    state.ui.view = View::PlaylistDetail;
    let track = Track::new("playlist", "Playlist track", "Artist");
    let mut playlist = Playlist::new("Snapshot playlist");
    playlist.tracks.push(PlaylistTrack::from(&track));
    state.domain.playlists.push(playlist);
    state.ui.selected_playlist = Some(0);

    let (_, text) = render_menu(&mut state);

    assert_order(
        &text,
        &[
            "Play now",
            "Play next",
            "Add to queue",
            "Add to playlist",
            "Visit channel",
            "Show details",
            "Open in browser",
            "Copy URL",
            "! Remove from playlist",
        ],
    );
}

#[test]
fn track_context_details_modal_renders_selected_track_without_changing_playback() {
    let mut state = AppState::new();
    let mut selected = Track::new("details", "Selected details", "Selected artist");
    selected.duration_seconds = Some(125);
    state.domain.current_track = Some(Track::new("playing", "Still playing", "Playback artist"));
    state.domain.playback.status = ytm_tui::playback::PlaybackStatus::Playing;
    state.ui.track_details_modal = Some(TrackDetailsModalState {
        track: selected,
        details: None,
    });

    let text = render_to_string(&mut state, None, 100, 30);

    assert!(text.contains("Selected details"), "selected title:\n{text}");
    assert!(text.contains("Selected artist"), "selected artist:\n{text}");
    assert!(text.contains("02:05"), "duration:\n{text}");
    assert!(text.contains("youtube.com/watch?v=details"), "URL:\n{text}");
    assert_eq!(
        state.domain.playback.status,
        ytm_tui::playback::PlaybackStatus::Playing
    );
}
