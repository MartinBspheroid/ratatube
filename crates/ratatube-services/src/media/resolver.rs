//! Stream URL resolution policy (PRD section 10.4).
//!
//! Canonical page URLs are stored; stream URLs are resolved shortly before
//! playback, re-resolved once on failure, and never persisted.

use crate::media::Track;
use crate::media::yt_dlp::YtDlp;
use ratatube_domain::error::Result;

/// Resolves temporary stream URLs on demand.
pub struct StreamResolver {
    yt_dlp: YtDlp,
}

/// Outcome of a playback attempt that needed resolution.
pub enum Resolution {
    /// Stream URL ready to hand to mpv.
    Ready(String),
    /// Track could not be resolved even after one retry.
    Unavailable,
}

impl StreamResolver {
    pub fn new(yt_dlp: YtDlp) -> Self {
        Self { yt_dlp }
    }

    /// Resolve the stream URL for a track's canonical page URL.
    pub async fn resolve(&self, track: &Track) -> Result<String> {
        self.yt_dlp.resolve_stream(&track.webpage_url).await
    }

    /// Re-resolve once after mpv reported an expired or rejected URL.
    pub async fn retry_once(&self, track: &Track) -> Resolution {
        match self.resolve(track).await {
            Ok(url) => Resolution::Ready(url),
            Err(err) => {
                tracing::warn!(?err, track = %track.id, "re-resolution failed");
                Resolution::Unavailable
            }
        }
    }
}
