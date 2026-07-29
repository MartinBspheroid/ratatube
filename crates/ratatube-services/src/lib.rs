//! The impure edge of ratatube: subprocesses, sockets, files, and the OS.
//!
//! Every module here is an adapter over a domain model: the model is
//! re-exported alongside its service so callers keep one vocabulary. Nothing
//! here renders or knows the UI exists.

pub mod config;
pub mod history;
pub mod media;
pub mod persistence;
pub mod platform;
pub mod playback;
pub mod playlists;
pub mod process;
pub mod queue;

pub use ratatube_domain::error;
