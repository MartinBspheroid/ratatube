//! Ratatui buffer snapshot tests (PRD section 22): empty states, search
//! results, queue, now-playing bar, ASCII mode, and narrow terminals.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use ratatube::app::state::AppState;
use ratatube::history::HistoryService;
use ratatube::history::model::{HistoryEntry, PlaybackOutcome};
use ratatube::media::Track;
use ratatube::media::search::SearchState;

#[path = "ui_snapshots/channel.rs"]
mod channel;
#[path = "ui_snapshots/context_menu.rs"]
mod context_menu;
#[path = "ui_snapshots/help.rs"]
mod help;
#[path = "ui_snapshots/history.rs"]
mod history;
#[path = "ui_snapshots/home.rs"]
mod home;
#[path = "ui_snapshots/layout.rs"]
mod layout;
#[path = "ui_snapshots/modals.rs"]
mod modals;
#[path = "ui_snapshots/playing.rs"]
mod playing;
#[path = "ui_snapshots/search.rs"]
mod search;

/// Render the app to a string for snapshot-style assertions.
fn render_to_string(
    state: &mut AppState,
    history: Option<&HistoryService>,
    w: u16,
    h: u16,
) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).expect("terminal");
    ratatube::ui::render_with(&mut terminal, state, history).expect("render");
    buffer_to_string(terminal.backend().buffer())
}

/// Flatten a ratatui buffer to a plain string (row-major, newline-separated).
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                out.push_str(cell.symbol());
            }
        }
        out.push('\n');
    }
    out
}
