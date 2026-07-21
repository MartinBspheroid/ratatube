//! ytm-tui binary: CLI entry point.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use ytm_tui::error::Result;
use ytm_tui::{app, config, persistence, process, queue};

/// Terminal YouTube Music player.
#[derive(Debug, Parser)]
#[command(name = "ytm-tui", version, about)]
struct Cli {
    /// Override all application data/config paths (useful for isolation).
    #[arg(long, global = true, value_name = "PATH")]
    data_dir: Option<PathBuf>,
    /// Restore the previous session and start playing immediately.
    #[arg(long)]
    resume: bool,
    /// Attach the TUI to the background service (experimental; some flows
    /// are not yet available while attached).
    #[arg(long)]
    attach: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Verify dependencies, paths, and configuration.
    Doctor,
    /// Search for a query (or URL) and play it via the background service.
    Play {
        /// Search terms or a YouTube URL.
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
    },
    /// Run the background playback service in the foreground.
    Daemon,
    /// Toggle pause on the background service.
    Pause,
    /// Stop playback on the background service.
    Stop,
    /// Show what the background service is playing.
    Status,
    /// Shut the background service down.
    Quit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = match cli.data_dir {
        Some(path) => persistence::AppPaths::with_data_dir(path),
        None => persistence::AppPaths::resolve()?,
    };

    match cli.command {
        Some(Command::Doctor) => run_doctor(&paths),
        Some(Command::Play { query }) => run_play(paths, query.join(" ")).await,
        Some(Command::Daemon) => run_daemon_mode(paths, cli.resume).await,
        Some(Command::Pause) => run_control(paths, ytm_tui::protocol::Command::PlayPause).await,
        Some(Command::Stop) => run_control(paths, ytm_tui::protocol::Command::Stop).await,
        Some(Command::Status) => run_status(paths).await,
        Some(Command::Quit) => run_quit(paths).await,
        None if cli.attach => run_attached(paths).await,
        None => {
            let intent = cli.resume.then_some(app::StartupIntent::Resume);
            run_tui(paths, intent).await
        }
    }
}

/// Check dependencies, storage, and config; print a readable report.
fn run_doctor(paths: &persistence::AppPaths) -> Result<()> {
    let mut ok = true;
    let config = match config::inspect(&paths.config_file()) {
        Ok(Some(config)) => {
            println!("OK   config          {}", paths.config_file().display());
            config
        }
        Ok(None) => {
            println!(
                "INFO config          defaults in use (no file at {})",
                paths.config_file().display()
            );
            config::Config::default()
        }
        Err(err) => {
            println!(
                "FAIL config          {} — {err}",
                paths.config_file().display()
            );
            println!("     recovery        fix or move the file; doctor made no changes");
            ok = false;
            config::Config::default()
        }
    };

    let probes = [
        process::probe("mpv", &config.paths.mpv),
        process::probe("yt-dlp", &config.paths.yt_dlp),
        process::probe("ffmpeg (optional)", "ffmpeg"),
        process::probe("curl (optional)", "curl"),
    ];
    for probe in &probes {
        match &probe.status {
            process::DependencyStatus::Found { path, version } => {
                println!(
                    "OK   {:<18} {} ({})",
                    probe.name,
                    path.display(),
                    version.as_deref().unwrap_or("version unknown")
                );
            }
            process::DependencyStatus::Missing => {
                let required = !probe.name.contains("optional");
                if required {
                    ok = false;
                }
                println!(
                    "{:<4} {:<18} missing — {}",
                    if required { "FAIL" } else { "WARN" },
                    probe.name,
                    process::install_hint(probe.binary.trim_end_matches(" (optional)"))
                );
            }
        }
    }

    if paths.data_dir.exists() {
        let data_ok = paths.data_dir.is_dir() && path_looks_writable(&paths.data_dir);
        println!(
            "{:<4} data dir        {}{}",
            if data_ok { "OK" } else { "FAIL" },
            paths.data_dir.display(),
            if data_ok {
                ""
            } else {
                " (not a writable directory)"
            }
        );
        ok &= data_ok;
    } else {
        println!(
            "INFO data dir        {} (not created yet)",
            paths.data_dir.display()
        );
        let parent_ok = paths.data_dir.parent().is_some_and(path_looks_writable);
        if !parent_ok {
            println!("FAIL parent dir      no writable parent for data directory");
            ok = false;
        }
    }

    println!(
        "{:<4} ipc path dir    {}",
        if paths.data_dir.is_dir() {
            "OK"
        } else {
            "INFO"
        },
        paths.data_dir.display()
    );
    if ok {
        println!("\nAll checks passed.");
        Ok(())
    } else {
        println!("\nSome checks failed; see hints above.");
        Err(ytm_tui::error::AppError::Config(
            "doctor checks failed".to_string(),
        ))
    }
}

/// Send a play request to the daemon (starting it if needed) and report the
/// resolved track once playback resolution lands.
async fn run_play(paths: persistence::AppPaths, query: String) -> Result<()> {
    let mut connection = ytm_tui::client::connect_or_spawn(&paths).await?;
    connection
        .request(ytm_tui::protocol::Command::PlayQuery {
            query: query.clone(),
        })
        .await?;
    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let ytm_tui::protocol::ReplyBody::Status { snapshot } = connection
            .request(ytm_tui::protocol::Command::Status)
            .await?
            && let Some(track) = snapshot.current_track
        {
            println!("Playing: {} — {}", track.title, track.artist);
            return Ok(());
        }
    }
    println!("Requested: {query} (still resolving; `ytm status` to check)");
    Ok(())
}

/// Send one control command to a running daemon (no auto-spawn).
async fn run_control(
    paths: persistence::AppPaths,
    command: ytm_tui::protocol::Command,
) -> Result<()> {
    let socket = ytm_tui::daemon::socket_path(&paths);
    let mut connection = ytm_tui::client::Connection::connect(&socket).await?;
    connection.request(command).await?;
    Ok(())
}

/// Print a human-readable report of the daemon's current state.
async fn run_status(paths: persistence::AppPaths) -> Result<()> {
    let socket = ytm_tui::daemon::socket_path(&paths);
    let mut connection = ytm_tui::client::Connection::connect(&socket).await?;
    let ytm_tui::protocol::ReplyBody::Status { snapshot } = connection
        .request(ytm_tui::protocol::Command::Status)
        .await?
    else {
        return Err(ytm_tui::error::AppError::Config(
            "daemon returned an unexpected status body".to_string(),
        ));
    };
    match &snapshot.current_track {
        Some(track) => {
            println!(
                "{:?}  {} — {}",
                snapshot.playback.status, track.title, track.artist
            );
            let position = snapshot.playback.position_seconds as u64;
            match snapshot.playback.duration_seconds {
                Some(total) => println!("  {position}s / {}s", total as u64),
                None => println!("  {position}s"),
            }
        }
        None => println!("Nothing is playing"),
    }
    println!(
        "  queue {} · volume {}%{}",
        snapshot.queue.tracks.len(),
        snapshot.playback.volume,
        if snapshot.health.mpv_ready {
            ""
        } else {
            " · mpv DOWN"
        }
    );
    Ok(())
}

/// Ask the daemon to shut down cleanly.
async fn run_quit(paths: persistence::AppPaths) -> Result<()> {
    let socket = ytm_tui::daemon::socket_path(&paths);
    let mut connection = ytm_tui::client::Connection::connect(&socket).await?;
    connection
        .request(ytm_tui::protocol::Command::Shutdown)
        .await?;
    println!("Daemon stopped.");
    Ok(())
}

/// Attach the TUI to the background service, auto-spawning it if needed.
/// The client keeps no local playback or persistence; the mirror hydrates
/// from the daemon's snapshot. Client-side tracing is skipped for now: the
/// daemon owns `ytm-tui.log` (a separate `ytm-ui.log` lands in phase 3).
async fn run_attached(paths: persistence::AppPaths) -> Result<()> {
    paths.ensure_dirs()?;
    let config = match config::load(&paths.config_file()) {
        Ok(config) => config,
        Err(err @ ytm_tui::error::AppError::MalformedData(_)) => {
            eprintln!("warning: {err}; continuing with default configuration");
            config::Config::default()
        }
        Err(err) => return Err(err),
    };
    let connection = ytm_tui::client::connect_or_spawn(&paths).await?;
    let state = app::state::AppState::new();
    let mut app = app::App::new(config, paths, state, Some(create_picker()));

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));
    let mut terminal = ratatui::init();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableBracketedPaste);
    let result = app.run_client(&mut terminal, connection).await;
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableBracketedPaste);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();
    result
}

/// Run the background service in the foreground (what auto-spawn executes).
async fn run_daemon_mode(paths: persistence::AppPaths, resume: bool) -> Result<()> {
    paths.ensure_dirs()?;
    if let Err(err) = init_tracing(&paths) {
        eprintln!("warning: logging unavailable: {err}");
    }
    let config = match config::load(&paths.config_file()) {
        Ok(config) => config,
        Err(err @ ytm_tui::error::AppError::MalformedData(_)) => {
            eprintln!("warning: {err}; continuing with default configuration");
            config::Config::default()
        }
        Err(err) => return Err(err),
    };
    let intent = resume.then_some(app::StartupIntent::Resume);
    ytm_tui::daemon::run(paths, config, intent).await
}

/// Launch the terminal UI.
async fn run_tui(paths: persistence::AppPaths, intent: Option<app::StartupIntent>) -> Result<()> {
    paths.ensure_dirs()?;
    if let Err(err) = init_tracing(&paths) {
        eprintln!("warning: logging unavailable: {err}");
    }
    // Malformed config must not lock the user out: report it and continue
    // with defaults (PRD 11.4); the original file is preserved with a .bak.
    let config = match config::load(&paths.config_file()) {
        Ok(config) => config,
        Err(err @ ytm_tui::error::AppError::MalformedData(_)) => {
            eprintln!("warning: {err}; continuing with default configuration");
            config::Config::default()
        }
        Err(err) => return Err(err),
    };

    // Restore persisted queue (PRD 10.5).
    let queue = queue::service::load(&paths.queue_file()).unwrap_or_else(|err| {
        tracing::warn!(?err, "queue restore failed; starting empty");
        queue::Queue::default()
    });
    let mut state = app::state::AppState::new().with_queue(queue);
    state.domain.yt_dlp_ready = process::require(&config.paths.yt_dlp).is_ok();

    let mut app = app::App::new(config, paths, state, Some(create_picker()));
    app.set_startup_intent(intent);
    app.load_initial_data();

    // Panic hook restores the terminal even on unexpected failure (PRD 26.12).
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    let mut terminal = ratatui::init();
    // Enable mouse events (click to select, wheel to scroll).
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableBracketedPaste);
    let result = app.run(&mut terminal).await;
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableBracketedPaste);
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();
    app.shutdown().await;
    result
}

/// Detect the terminal graphics protocol.
///
/// - Inside a multiplexer (herdr/tmux) graphics escapes don't pass through,
///   so use halfblocks, which render anywhere.
/// - Ghostty/Kitty/WezTerm answer capability queries instantly, giving the
///   real font size so images fit their cells exactly.
/// - Non-interactive contexts (pipes, tests) use halfblocks without
///   querying, since a query would consume buffered input and stall.
fn create_picker() -> ratatui_image::picker::Picker {
    use ratatui_image::picker::{Picker, ProtocolType};

    let inside_mux = std::env::var_os("TMUX").is_some() || std::env::var_os("HERDR_ENV").is_some();
    if inside_mux {
        return Picker::halfblocks();
    }

    let kitty_capable = std::env::var("TERM_PROGRAM")
        .map(|v| matches!(v.as_str(), "ghostty" | "iTerm.app" | "WezTerm" | "kitty"))
        .unwrap_or(false)
        || std::env::var("TERM")
            .map(|v| v.contains("kitty") || v.contains("ghostty"))
            .unwrap_or(false);

    if std::io::IsTerminal::is_terminal(&std::io::stdin())
        && let Ok(picker) = Picker::from_query_stdio()
    {
        return picker;
    }

    if kitty_capable {
        // The query failed but the terminal is known to support graphics;
        // force the protocol with a font size derived from the window size.
        let mut picker = picker_from_window_size();
        let proto = if std::env::var("TERM_PROGRAM").as_deref() == Ok("iTerm.app") {
            ProtocolType::Iterm2
        } else {
            ProtocolType::Kitty
        };
        picker.set_protocol_type(proto);
        picker
    } else {
        Picker::halfblocks()
    }
}

/// Build a picker whose font size matches the actual terminal cell size.
/// `from_fontsize` is deprecated in v11 but is the only way to set the font
/// size without a stdin query; suppress the lint with this explanation.
#[allow(deprecated)]
fn picker_from_window_size() -> ratatui_image::picker::Picker {
    use ratatui_image::picker::Picker;
    match crossterm::terminal::window_size() {
        Ok(size) if size.columns > 0 && size.rows > 0 => {
            Picker::from_fontsize(ratatui_image::FontSize::new(
                (size.width / size.columns).max(1),
                (size.height / size.rows).max(1),
            ))
        }
        _ => Picker::halfblocks(),
    }
}

/// Route logs to a local file; never to the terminal UI (PRD 16).
fn init_tracing(paths: &persistence::AppPaths) -> Result<()> {
    let file = ytm_tui::diagnostics::open_log(&paths.log_file())?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false)
        .try_init()
        .map_err(|err| ytm_tui::error::AppError::Config(format!("tracing init failed: {err}")))?;
    Ok(())
}

fn path_looks_writable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o222 != 0
    }
    #[cfg(not(unix))]
    {
        !metadata.permissions().readonly()
    }
}
