//! Playback-history service over the domain history model.

pub mod service;

pub use ratatube_domain::history::{activity, model};
pub use service::HistoryService;
