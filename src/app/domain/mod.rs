//! Domain half of the app layer: supervised external work (yt-dlp, mpv),
//! persistence submission, and operation identity. This is the code the
//! future daemon owns; it must stay free of terminal/rendering types.

pub(in crate::app) mod background;
pub mod channel;
pub(in crate::app) mod effects;
pub(in crate::app) mod media_tasks;
pub mod operations;
pub(in crate::app) mod persistence;
pub(in crate::app) mod playback_followup;
pub(in crate::app) mod playback_recovery;
pub(in crate::app) mod playback_session;
