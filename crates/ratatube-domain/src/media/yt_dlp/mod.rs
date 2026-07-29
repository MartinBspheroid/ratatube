//! Normalization of raw yt-dlp JSON into domain media types. The subprocess
//! client that produces that JSON is a service and lives outside this crate.

mod entry;
mod types;

pub use entry::YtDlpEntry;
pub use types::{ImportRejections, PlaylistFetch, SkipReason};
