//! Media discovery, metadata, and stream resolution via yt-dlp.

pub mod channel;
mod chapters;
mod details;
mod thumbnail;
mod track;

pub mod import;
pub mod resolver;
pub mod search;
pub mod yt_dlp;

pub use chapters::{Chapter, chapter_at, parse_chapters_from_description};
pub use details::{TrackDetails, format_count};
pub use thumbnail::decode_thumbnail;
pub use track::{Availability, Track};
