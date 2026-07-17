//! ytm-tui binary: CLI entry point.

use clap::{Parser, Subcommand};
use ytm_tui::error::Result;
use ytm_tui::{app, config, persistence, process, queue};

/// Terminal YouTube Music player.
#[derive(Debug, Parser)]
#[command(name = "ytm-tui", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Verify dependencies, paths, and configuration.
    Doctor,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = persistence::AppPaths::resolve()?;
    paths.ensure_dirs()?;
    init_tracing(&paths);

    match cli.command {
        Some(Command::Doctor) => run_doctor(&paths),
        None => run_tui(paths).await,
    }
}

/// Check dependencies, storage, and config; print a readable report.
fn run_doctor(paths: &persistence::AppPaths) -> Result<()> {
    let config = config::load(&paths.config_file())?;

    let probes = [
        process::probe("mpv", &config.paths.mpv),
        process::probe("yt-dlp", &config.paths.yt_dlp),
        process::probe("ffmpeg (optional)", "ffmpeg"),
    ];
    let mut ok = true;
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

    let data_ok = paths.data_dir.exists() && paths.playlists_dir().exists();
    println!(
        "{:<4} data dir        {}",
        if data_ok { "OK" } else { "FAIL" },
        paths.data_dir.display()
    );
    ok &= data_ok;

    let ipc_dir_ok = paths.data_dir.is_dir();
    println!(
        "{:<4} ipc path dir    {}",
        if ipc_dir_ok { "OK" } else { "FAIL" },
        paths.data_dir.display()
    );
    ok &= ipc_dir_ok;

    println!("OK   config          {}", paths.config_file().display());
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

/// Launch the terminal UI.
async fn run_tui(paths: persistence::AppPaths) -> Result<()> {
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
    state.yt_dlp_ready = process::require(&config.paths.yt_dlp).is_ok();

    let mut app = app::App::new(config, paths, state, create_picker());
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
    let result = app.run(&mut terminal).await;
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();
    app.shutdown().await;
    result
}

/// Detect the terminal graphics protocol. Ghostty/Kitty/WezTerm are
/// detected from environment variables so no stdin query is needed (a query
/// would consume buffered input and stall on terminals that don't answer).
/// Other interactive terminals get a capability query; anything else falls
/// back to halfblocks, which render everywhere.
fn create_picker() -> ratatui_image::picker::Picker {
    use ratatui_image::picker::{Picker, ProtocolType};

    let kitty_capable = std::env::var("TERM_PROGRAM")
        .map(|v| matches!(v.as_str(), "ghostty" | "iTerm.app" | "WezTerm" | "kitty"))
        .unwrap_or(false)
        || std::env::var("TERM")
            .map(|v| v.contains("kitty") || v.contains("ghostty"))
            .unwrap_or(false);

    if kitty_capable {
        let mut picker = Picker::halfblocks();
        let proto = if std::env::var("TERM_PROGRAM").as_deref() == Ok("iTerm.app") {
            ProtocolType::Iterm2
        } else {
            ProtocolType::Kitty
        };
        picker.set_protocol_type(proto);
        return picker;
    }

    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
    } else {
        Picker::halfblocks()
    }
}

/// Route logs to a local file; never to the terminal UI (PRD 16).
fn init_tracing(paths: &persistence::AppPaths) {
    let file = std::fs::File::create(paths.log_file());
    let Ok(file) = file else {
        return;
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false)
        .init();
}
