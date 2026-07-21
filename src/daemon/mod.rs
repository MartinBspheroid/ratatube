//! Background service entry: single-instance socket ownership and the
//! headless runtime (see the phase 2 plan and design spec).

pub(crate) mod server;

use std::os::unix::fs::PermissionsExt;

use tokio::net::{UnixListener, UnixStream};

use crate::app::{App, StartupIntent};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::persistence::AppPaths;

/// Control socket name inside the data directory.
pub const SOCKET_NAME: &str = "ytm.sock";
/// Best-effort pid record for `doctor`; the socket itself is the lock.
pub const PID_FILE_NAME: &str = "daemon.pid";

/// Path of the daemon control socket for a data directory.
pub fn socket_path(paths: &AppPaths) -> std::path::PathBuf {
    paths.data_dir.join(SOCKET_NAME)
}

/// Run the daemon until a `Shutdown` command or SIGTERM-equivalent quit.
///
/// The bound socket is the single-instance lock: binding fails while a live
/// daemon owns it; a dead daemon's stale socket is detected by a failed
/// probe connect and removed.
pub async fn run(paths: AppPaths, config: Config, intent: Option<StartupIntent>) -> Result<()> {
    paths.ensure_dirs()?;
    let socket = socket_path(&paths);
    if UnixStream::connect(&socket).await.is_ok() {
        return Err(AppError::Config(format!(
            "a daemon is already running on {}",
            socket.display()
        )));
    }
    if socket.exists() {
        std::fs::remove_file(&socket).map_err(|err| {
            AppError::Config(format!(
                "could not remove stale socket {}: {err}",
                socket.display()
            ))
        })?;
    }
    let listener = UnixListener::bind(&socket)
        .map_err(|err| AppError::Config(format!("could not bind {}: {err}", socket.display())))?;
    let _ = std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600));
    let pid_file = paths.data_dir.join(PID_FILE_NAME);
    let _ = std::fs::write(&pid_file, format!("{}\n", std::process::id()));

    let mut app = build_app(paths, config);
    app.set_startup_intent(intent);
    app.load_initial_data();
    let result = app.run_daemon(listener).await;
    app.shutdown().await;
    let _ = std::fs::remove_file(&socket);
    let _ = std::fs::remove_file(&pid_file);
    result
}

/// Construct the headless app (no terminal graphics picker).
fn build_app(paths: AppPaths, config: Config) -> App {
    let queue = crate::queue::service::load(&paths.queue_file()).unwrap_or_else(|err| {
        tracing::warn!(?err, "queue restore failed; starting empty");
        crate::queue::Queue::default()
    });
    let mut state = crate::app::state::AppState::new().with_queue(queue);
    state.domain.yt_dlp_ready = crate::process::require(&config.paths.yt_dlp).is_ok();
    App::new(config, paths, state, None)
}
