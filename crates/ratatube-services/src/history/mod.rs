//! Playback-history service over the domain history model.

pub mod service;

pub use ratatube_domain::history::{HistoryDocument, HistoryLog, TrackStats, activity, log, model};
pub use service::HistoryService;
