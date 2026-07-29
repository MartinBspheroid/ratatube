//! ratatube: terminal YouTube Music player library.
//!
//! The binary in `main.rs` is a thin shell over the workspace: the pure core
//! is `ratatube-domain`, the impure edge is `ratatube-services`, the wire is
//! `ratatube-protocol`. What stays here is the application itself — state
//! composition, reduction, effect execution, rendering, and the two runtimes.

pub mod app;
pub mod client;
pub mod daemon;
pub mod diagnostics;
pub use ratatube_ui::{input, render as ui};

pub use ratatube_domain::error;
pub use ratatube_protocol as protocol;
pub use ratatube_services::{
    config, history, media, persistence, platform, playback, playlists, process, queue,
};
