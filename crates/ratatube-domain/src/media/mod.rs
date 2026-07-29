//! Media identity, metadata, and the normalization of external media JSON.

pub mod channel;
mod chapters;
mod details;
pub mod import;
pub mod search;
mod track;
pub mod yt_dlp;

pub use chapters::{Chapter, chapter_at, parse_chapters_from_description};
pub use details::{TrackDetails, format_count};
pub use track::{Availability, Track};
