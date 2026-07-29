//! Pure domain core for ratatube: state, per-context commands, effects, and
//! change events.
//!
//! This crate is deliberately free of tokio, ratatui, and every process or
//! terminal dependency. Nothing here renders, spawns, blocks, or touches the
//! filesystem; the impure edge lives in the binary's service layer. CI
//! asserts the dependency tree stays clean.

pub mod commands;
pub mod config;
pub mod effect;
pub mod error;
pub mod event;
pub mod history;
pub mod media;
pub mod operations;
pub mod playback;
pub mod playlists;
pub mod queue;
pub mod resume;
pub mod schema;
pub mod state;
