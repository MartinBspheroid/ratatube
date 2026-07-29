//! Application orchestration tests split by responsibility.

mod browser;
mod command_path;
#[cfg(unix)]
mod fake_executable;
mod mouse;
mod playlist_workflows;
mod selection;
mod session;

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, PlaybackAction, PlaylistAction, QueueAction};
use crate::app::state::{AppState, ImportState, PromptPurpose, View};
use crate::config::Config;
use crate::history::HistoryService;
use crate::history::model::PlaybackOutcome;
use crate::media::Track;
use crate::persistence::AppPaths;
use crate::playback::PlaybackEvent;
use crate::playlists::Playlist;

/// Build an isolated application fixture backed by a temporary data directory.
pub(super) fn test_app() -> (tempfile::TempDir, App) {
    let temp = tempfile::tempdir().expect("tempdir");
    let paths = AppPaths::with_data_dir(temp.path().to_path_buf());
    paths.ensure_dirs().expect("create test data root");
    let app = App::new(
        Config::default(),
        paths,
        AppState::new(),
        Some(ratatui_image::picker::Picker::halfblocks()),
    );
    (temp, app)
}
