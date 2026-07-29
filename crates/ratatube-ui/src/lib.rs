//! The client half of ratatube: UI state, presentation reducers, terminal
//! rendering, and input mapping.
//!
//! This crate reads `ratatube-domain` and never writes it: domain change
//! events arrive through [`sync::apply_domain_events`], the single legal
//! reaction path. It has no socket, no subprocess, and no persistence.

pub mod filter;
pub mod input;
pub mod reducer;
pub mod render;
pub mod state;
pub mod sync;
