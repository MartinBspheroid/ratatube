use crate::media::Track;

/// Exact reason an external playlist entry was not importable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The yt-dlp entry did not contain a video identifier.
    MissingId,
    /// The yt-dlp entry did not contain a title.
    MissingTitle,
    /// The yt-dlp entry represents a deleted video.
    Deleted,
    /// The yt-dlp entry represents a private video.
    Private,
    /// The yt-dlp entry represents an unavailable video.
    Unavailable,
}

/// Traceable rejection counts from yt-dlp normalization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportRejections {
    /// Lines that were not valid yt-dlp JSON objects.
    pub malformed: usize,
    /// Entries without a video identifier.
    pub missing_id: usize,
    /// Entries without a title.
    pub missing_title: usize,
    /// Deleted entries.
    pub deleted: usize,
    /// Private entries.
    pub private: usize,
    /// Unavailable entries.
    pub unavailable: usize,
}

impl ImportRejections {
    pub fn record(&mut self, reason: SkipReason) {
        match reason {
            SkipReason::MissingId => self.missing_id += 1,
            SkipReason::MissingTitle => self.missing_title += 1,
            SkipReason::Deleted => self.deleted += 1,
            SkipReason::Private => self.private += 1,
            SkipReason::Unavailable => self.unavailable += 1,
        }
    }

    pub fn record_malformed(&mut self) {
        self.malformed += 1;
    }

    /// Return the total number of rejected entries.
    pub fn total(&self) -> usize {
        self.malformed
            + self.missing_id
            + self.missing_title
            + self.deleted
            + self.private
            + self.unavailable
    }
}

/// Result of a flat playlist fetch, including skipped entries (PRD 10.8).
#[derive(Debug)]
pub struct PlaylistFetch {
    /// The playlist title reported by yt-dlp.
    pub title: String,
    /// The remote playlist identifier, when yt-dlp reported one.
    pub remote_id: Option<String>,
    /// Playable tracks normalized from playlist entries.
    pub tracks: Vec<Track>,
    /// Entries that could not be normalized.
    pub rejections: ImportRejections,
}
