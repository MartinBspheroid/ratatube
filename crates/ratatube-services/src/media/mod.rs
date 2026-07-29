//! Media services: the yt-dlp client, stream resolution, and thumbnails.

mod thumbnail;

pub mod resolver;
pub mod yt_dlp;

pub use ratatube_domain::media::{
    Availability, Chapter, Track, TrackDetails, channel, chapter_at, format_count, import,
    parse_chapters_from_description, search,
};
pub use thumbnail::decode_thumbnail;
