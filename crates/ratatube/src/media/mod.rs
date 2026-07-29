//! Media services: the yt-dlp client, stream resolution, and thumbnails.
//!
//! The media *model* (tracks, chapters, details, search state, channel pages,
//! URL classification) lives in `ratatube-domain` and is re-exported here so
//! the service layer keeps one media vocabulary.

mod thumbnail;

pub mod resolver;
pub mod yt_dlp;

pub use ratatube_domain::media::{
    Availability, Chapter, Track, TrackDetails, channel, chapter_at, format_count, import,
    parse_chapters_from_description, search,
};
pub use thumbnail::decode_thumbnail;
