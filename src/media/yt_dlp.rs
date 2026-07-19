//! yt-dlp subprocess client.
//!
//! yt-dlp processes are short-lived per operation (PRD section 14). All
//! invocations use direct argument execution without a shell (PRD 19).

mod entry;
mod process;
mod types;

pub(crate) use entry::YtDlpEntry;

use crate::error::{AppError, Result};
use crate::media::Track;
use crate::media::channel::{ChannelPage, ChannelPageRequest, parse_channel_page};

pub use types::{ImportRejections, PlaylistFetch, SkipReason};

const DEFAULT_STDOUT_LIMIT: usize = 16 * 1024 * 1024;
const DEFAULT_STDERR_LIMIT: usize = 1024 * 1024;

/// Async client wrapping the yt-dlp executable.
#[derive(Debug, Clone)]
pub struct YtDlp {
    binary: String,
    stdout_limit: usize,
    stderr_limit: usize,
}

impl YtDlp {
    /// Create a client that invokes the supplied yt-dlp executable.
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            stdout_limit: DEFAULT_STDOUT_LIMIT,
            stderr_limit: DEFAULT_STDERR_LIMIT,
        }
    }

    /// Override subprocess output limits (primarily for deterministic tests).
    pub fn with_output_limits(mut self, stdout_limit: usize, stderr_limit: usize) -> Self {
        self.stdout_limit = stdout_limit;
        self.stderr_limit = stderr_limit;
        self
    }

    /// Search YouTube, returning up to `limit` metadata-only results.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        let spec = format!("ytsearch{limit}:{query}");
        let output = self
            .run(&[
                "--dump-json",
                "--flat-playlist",
                "--no-download",
                "--ignore-errors",
                "--",
                &spec,
            ])
            .await?;
        let mut tracks = Vec::new();
        for line in output.lines().filter(|line| !line.trim().is_empty()) {
            match serde_json::from_str::<YtDlpEntry>(line) {
                Ok(entry) => {
                    if let Ok(track) = entry.into_track() {
                        tracks.push(track);
                    }
                }
                Err(error) => tracing::warn!(?error, "skipping unparseable yt-dlp line"),
            }
        }
        Ok(tracks)
    }

    /// Fetch full metadata for a single video URL.
    pub async fn fetch_video(&self, url: &str) -> Result<Track> {
        let output = self.run(&["-J", "--no-download", "--", url]).await?;
        let entry: YtDlpEntry = serde_json::from_str(&output)?;
        entry
            .into_track()
            .map_err(|reason| AppError::Resolve(format!("{reason:?} for {url}")))
    }

    /// Fetch extended metadata for the detail panel.
    pub async fn fetch_details(&self, url: &str) -> Result<crate::media::TrackDetails> {
        let output = self.run(&["-J", "--no-download", "--", url]).await?;
        let entry: YtDlpEntry = serde_json::from_str(&output)?;
        Ok(entry.into_details())
    }

    /// Fetch all metadata-only entries for a playlist URL.
    pub async fn fetch_playlist(&self, url: &str) -> Result<PlaylistFetch> {
        let output = self
            .run(&["-J", "--flat-playlist", "--no-download", "--", url])
            .await?;
        let root: YtDlpEntry = serde_json::from_str(&output)?;
        let mut tracks = Vec::new();
        let mut rejections = ImportRejections::default();
        for entry in root.entries {
            match entry.into_track() {
                Ok(track) => tracks.push(track),
                Err(reason) => rejections.record(reason),
            }
        }
        Ok(PlaylistFetch {
            title: root.title.unwrap_or_else(|| "Playlist".to_string()),
            remote_id: root.id,
            rejections,
            tracks,
        })
    }

    /// Fetch one bounded, newest-first page from a YouTube channel's videos.
    pub async fn fetch_channel_page(&self, request: &ChannelPageRequest) -> Result<ChannelPage> {
        let (start, end) = request.bounds()?;
        let start = start.to_string();
        let end = end.to_string();
        let url = request.videos_url()?;
        let output = self
            .run(&[
                "--dump-json",
                "--flat-playlist",
                "--no-download",
                "--ignore-errors",
                "--playlist-start",
                &start,
                "--playlist-end",
                &end,
                "--",
                &url,
            ])
            .await?;
        Ok(parse_channel_page(&output))
    }

    /// Fetch YouTube's auto-generated mix (radio) for a seed video.
    pub async fn fetch_mix(&self, video_id: &str) -> Result<PlaylistFetch> {
        let url = format!("https://www.youtube.com/watch?v={video_id}&list=RD{video_id}");
        self.fetch_playlist(&url).await
    }

    /// Resolve a temporary audio stream URL for playback.
    ///
    /// The returned URL is runtime-only state and must never be persisted.
    pub async fn resolve_stream(&self, webpage_url: &str) -> Result<String> {
        let output = self
            .run(&[
                "-f",
                "bestaudio/best",
                "-g",
                "--no-download",
                "--",
                webpage_url,
            ])
            .await?;
        let url = output.lines().next().unwrap_or_default().trim().to_string();
        if url.is_empty() {
            return Err(AppError::Resolve(format!("no stream for {webpage_url}")));
        }
        Ok(url)
    }

    /// Run yt-dlp with a timeout, returning stdout on success.
    async fn run(&self, args: &[&str]) -> Result<String> {
        process::run(&self.binary, args, self.stdout_limit, self.stderr_limit).await
    }
}
