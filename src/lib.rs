//! ratatube: terminal YouTube Music player library.
//!
//! The binary in `main.rs` is a thin shell over these modules: audio
//! playback via persistent mpv, media discovery via yt-dlp, local JSON
//! persistence. See the PRD for the full scope.

pub mod app;
pub mod client;
pub mod config;
pub mod daemon;
pub mod diagnostics;
pub mod error;
pub mod history;
pub mod input;
pub mod media;
pub mod persistence;
pub mod platform;
pub mod playback;
pub mod playlists;
pub mod process;
pub mod protocol;
pub mod queue;
pub mod ui;
