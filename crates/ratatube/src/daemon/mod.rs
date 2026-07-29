//! Background service entry: single-instance socket ownership and the
//! headless runtime (see the phase 2 plan and design spec).

pub(crate) mod server;

use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tokio::net::{UnixListener, UnixStream};

use crate::app::{App, StartupIntent};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::persistence::AppPaths;

/// Control socket name inside the data directory.
pub const SOCKET_NAME: &str = "ratatube.sock";
/// Best-effort pid record for `doctor`; the socket itself is the lock.
pub const PID_FILE_NAME: &str = "daemon.pid";

/// Path of the daemon control socket for a data directory.
pub fn socket_path(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join(SOCKET_NAME)
}

/// Run the daemon until a `Shutdown` command or SIGTERM-equivalent quit.
///
/// The published socket name is the single-instance lock: claiming it fails
/// while a live daemon owns it; only a socket that provably has no listener
/// is treated as a dead daemon's leftover and replaced.
pub async fn run(paths: AppPaths, config: Config, intent: Option<StartupIntent>) -> Result<()> {
    paths.ensure_dirs()?;
    let socket = socket_path(&paths);
    let listener = claim_socket(&socket).await?;
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

/// Take exclusive ownership of the control socket path, or refuse.
///
/// A probe connect classifies the path, and only one outcome permits
/// unlinking it. `ECONNREFUSED` means the name exists but nothing is
/// listening — a leftover from a crashed daemon, safe to replace.
/// `ENOENT` means the name is free. Every other error (a live daemon whose
/// accept backlog is momentarily full answers `EAGAIN`; a directory we may
/// not traverse answers `EACCES`) is reported as-is: the old code unlinked on
/// *any* probe failure, which let a second daemon steal a live daemon's
/// socket and leave two processes writing the same JSON documents.
async fn claim_socket(socket: &Path) -> Result<UnixListener> {
    match UnixStream::connect(socket).await {
        Ok(_) => {
            return Err(AppError::Config(format!(
                "a daemon is already running on {}",
                socket.display()
            )));
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) if err.kind() == ErrorKind::ConnectionRefused => {
            std::fs::remove_file(socket).map_err(|err| {
                AppError::Config(format!(
                    "could not remove stale socket {}: {err}",
                    socket.display()
                ))
            })?;
        }
        Err(err) => {
            return Err(AppError::Config(format!(
                "could not determine whether a daemon owns {} ({err}); \
                 refusing to start rather than displace a possibly live daemon",
                socket.display()
            )));
        }
    }
    bind_owner_only(socket)
}

/// Bind the listener so the socket is never reachable with permissions wider
/// than 0o600.
///
/// Binding directly and chmod-ing afterwards leaves a window in which the
/// socket carries umask-derived permissions and any local user can connect.
/// Instead the listener binds a staging name in the same directory, is
/// restricted there, and only then gets its published name — the binding
/// follows the inode, so publishing is a directory operation on an already
/// protected socket.
///
/// `hard_link` publishes, not `rename`: it is equally atomic but fails with
/// `EEXIST` if the socket path reappeared since the probe, which preserves
/// the mutual exclusion `bind` used to give us. A `rename` would silently
/// clobber a socket a concurrently starting daemon had just published, so it
/// would trade the permissions window for a second-daemon window.
///
/// A failed chmod is fatal, unlike the old `let _ = set_permissions(..)`.
/// This socket accepts every daemon command — playlist mutation, process
/// spawning, playback control — so a control channel we cannot prove is
/// owner-only is worse for the user than no daemon at all.
fn bind_owner_only(socket: &Path) -> Result<UnixListener> {
    let staging = staging_path(socket);
    let _ = std::fs::remove_file(&staging);
    let listener = UnixListener::bind(&staging)
        .map_err(|err| AppError::Config(format!("could not bind {}: {err}", staging.display())))?;
    let published = std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o600))
        .map_err(|err| {
            AppError::Config(format!(
                "could not restrict {} to owner-only ({err}); \
                 refusing to expose an unprotected control socket",
                staging.display()
            ))
        })
        .and_then(|()| {
            std::fs::hard_link(&staging, socket).map_err(|err| {
                AppError::Config(format!(
                    "could not publish the control socket at {} ({err})",
                    socket.display()
                ))
            })
        });
    // The staging name has served its purpose either way; the listener keeps
    // the socket alive through its file descriptor.
    let _ = std::fs::remove_file(&staging);
    published.map(|()| listener)
}

/// Staging name for a socket about to be published: the same directory, so
/// the link into place stays on one filesystem, and a name derived from the
/// pid, so two daemons racing on one data directory cannot share it.
///
/// The name is deliberately kept shorter than `SOCKET_NAME`. A bound socket
/// path must fit `SUN_LEN` (108 bytes), so a longer staging name would make
/// deep `--data-dir` paths fail to bind that previously worked — the hex pid
/// is at most 8 characters plus the dot.
fn staging_path(socket: &Path) -> PathBuf {
    let dir = socket.parent().unwrap_or_else(|| Path::new("."));
    debug_assert!(format!(".{:x}", std::process::id()).len() <= SOCKET_NAME.len());
    dir.join(format!(".{:x}", std::process::id()))
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
