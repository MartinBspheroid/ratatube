//! yt-dlp subprocess client.
//!
//! yt-dlp processes are short-lived per operation (PRD section 14). All
//! invocations use direct argument execution without a shell (PRD 19).

use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::process::Command;

use crate::error::{AppError, Result};
use crate::media::Track;

/// Default timeout for a metadata operation.
const METADATA_TIMEOUT: Duration = Duration::from_secs(60);

/// Raw yt-dlp JSON shape for a single entry (subset of fields we use).
#[derive(Debug, Deserialize)]
struct YtDlpEntry {
    id: Option<String>,
    title: Option<String>,
    /// yt-dlp emits both `uploader` and `channel`; coalesce on use.
    uploader: Option<String>,
    channel: Option<String>,
    webpage_url: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    availability: Option<String>,
    #[serde(default)]
    entries: Vec<YtDlpEntry>,
    // Extended metadata, present on full (`-J`) single-video fetches.
    description: Option<String>,
    view_count: Option<u64>,
    like_count: Option<u64>,
    upload_date: Option<String>,
    #[serde(default)]
    categories: Vec<Option<String>>,
    #[serde(default)]
    chapters: Option<Vec<YtDlpChapter>>,
}

/// Raw yt-dlp chapter entry.
#[derive(Debug, Deserialize)]
struct YtDlpChapter {
    start_time: Option<f64>,
    title: Option<String>,
}

impl YtDlpEntry {
    /// Normalize into a [`Track`]; returns `None` when required metadata is
    /// missing (PRD 10.8 skipped entries).
    fn into_track(self) -> Option<Track> {
        let id = self.id?;
        let title = self.title?;
        if title == "[Deleted video]" || title == "[Private video]" {
            return None;
        }
        let availability = match self.availability.as_deref() {
            Some("private") => crate::media::Availability::Private,
            Some("unavailable") => crate::media::Availability::Unavailable,
            Some(_) => crate::media::Availability::Available,
            None => crate::media::Availability::Unknown,
        };
        Some(Track {
            webpage_url: self
                .webpage_url
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}")),
            id,
            title,
            artist: self
                .uploader
                .or(self.channel)
                .unwrap_or_else(|| "Unknown".to_string()),
            duration_seconds: self.duration.map(|d| d.max(0.0) as u64),
            thumbnail_url: self.thumbnail,
            availability,
        })
    }
}

/// Result of a flat playlist fetch, including skipped entries (PRD 10.8).
#[derive(Debug)]
pub struct PlaylistFetch {
    pub title: String,
    pub remote_id: Option<String>,
    pub tracks: Vec<Track>,
    /// Entries that could not be normalized (deleted, private, missing data).
    pub skipped: usize,
}

/// Async client wrapping the yt-dlp executable.
#[derive(Debug, Clone)]
pub struct YtDlp {
    binary: String,
}

impl YtDlp {
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
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
                &spec,
            ])
            .await?;
        let mut tracks = Vec::new();
        // --dump-json emits one JSON object per line.
        for line in output.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<YtDlpEntry>(line) {
                Ok(entry) => {
                    if let Some(track) = entry.into_track() {
                        tracks.push(track);
                    }
                }
                Err(err) => {
                    // Partial results are kept; unparseable lines are logged
                    // (PRD 10.1 partial results requirement).
                    tracing::warn!(?err, "skipping unparseable yt-dlp line");
                }
            }
        }
        Ok(tracks)
    }

    /// Fetch full metadata for a single video URL.
    pub async fn fetch_video(&self, url: &str) -> Result<Track> {
        let output = self.run(&["-J", "--no-download", url]).await?;
        let entry: YtDlpEntry = serde_json::from_str(&output)?;
        entry
            .into_track()
            .ok_or_else(|| AppError::Resolve(format!("missing metadata for {url}")))
    }

    /// Fetch extended metadata (description, stats, chapters) for the
    /// detail panel. When the uploader set no chapters, a tracklist parsed
    /// from the description fills in (DJ mixes).
    pub async fn fetch_details(&self, url: &str) -> Result<crate::media::TrackDetails> {
        let output = self.run(&["-J", "--no-download", url]).await?;
        let entry: YtDlpEntry = serde_json::from_str(&output)?;
        let mut chapters: Vec<crate::media::Chapter> = entry
            .chapters
            .unwrap_or_default()
            .into_iter()
            .filter_map(|c| {
                Some(crate::media::Chapter {
                    start_seconds: c.start_time?,
                    title: c.title.unwrap_or_default(),
                })
            })
            .collect();
        if chapters.is_empty()
            && let Some(description) = entry.description.as_deref()
        {
            chapters = crate::media::parse_chapters_from_description(description);
        }
        Ok(crate::media::TrackDetails {
            description: entry.description,
            view_count: entry.view_count,
            like_count: entry.like_count,
            upload_date: entry.upload_date,
            uploader: entry.uploader.or(entry.channel),
            categories: entry.categories.into_iter().flatten().collect(),
            chapters,
        })
    }

    /// Fetch all entries of a playlist URL, metadata only.
    pub async fn fetch_playlist(&self, url: &str) -> Result<PlaylistFetch> {
        let output = self
            .run(&["-J", "--flat-playlist", "--no-download", url])
            .await?;
        let root: YtDlpEntry = serde_json::from_str(&output)?;
        let total = root.entries.len();
        let tracks: Vec<Track> = root
            .entries
            .into_iter()
            .filter_map(YtDlpEntry::into_track)
            .collect();
        Ok(PlaylistFetch {
            title: root.title.clone().unwrap_or_else(|| "Playlist".to_string()),
            remote_id: root.id.clone(),
            skipped: total - tracks.len(),
            tracks,
        })
    }

    /// Fetch YouTube's auto-generated mix ("radio") for a seed video,
    /// metadata only. Used for radio mode and RD* mix URLs.
    pub async fn fetch_mix(&self, video_id: &str) -> Result<PlaylistFetch> {
        let url = format!("https://www.youtube.com/watch?v={video_id}&list=RD{video_id}");
        self.fetch_playlist(&url).await
    }

    /// Resolve a temporary audio stream URL for playback (PRD 10.4).
    ///
    /// The returned URL is runtime-only state and must never be persisted.
    pub async fn resolve_stream(&self, webpage_url: &str) -> Result<String> {
        let output = self
            .run(&["-f", "bestaudio/best", "-g", "--no-download", webpage_url])
            .await?;
        let url = output.lines().next().unwrap_or_default().trim().to_string();
        if url.is_empty() {
            return Err(AppError::Resolve(format!("no stream for {webpage_url}")));
        }
        Ok(url)
    }

    /// Run yt-dlp with a timeout, returning stdout on success.
    async fn run(&self, args: &[&str]) -> Result<String> {
        let mut command = Command::new(&self.binary);
        command
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            // Prevent orphan processes (PRD section 14).
            .kill_on_drop(true);

        let child = command.spawn().map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppError::MissingDependency(self.binary.clone())
            } else {
                AppError::Process {
                    command: self.binary.clone(),
                    message: err.to_string(),
                }
            }
        })?;

        let result = tokio::time::timeout(METADATA_TIMEOUT, child.wait_with_output()).await;
        match result {
            Ok(Ok(output)) if output.status.success() => {
                Ok(String::from_utf8_lossy(&output.stdout).into_owned())
            }
            Ok(Ok(output)) => Err(AppError::YtDlp(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            )),
            Ok(Err(err)) => Err(AppError::Io(err)),
            Err(_) => Err(AppError::Timeout(format!(
                "yt-dlp exceeded {}s",
                METADATA_TIMEOUT.as_secs()
            ))),
        }
    }
}

#[cfg(test)]
mod parse_tests {
    use super::YtDlpEntry;

    /// Regression: real yt-dlp flat-playlist entries contain both `uploader`
    /// and `channel`; parsing must not treat them as a duplicate field.
    #[test]
    fn parses_flat_entry_with_uploader_and_channel() {
        let line = r#"{"_type":"url","id":"u7K72X4eo_s","url":"https://www.youtube.com/watch?v=u7K72X4eo_s","title":"Massive Attack - Teardrop (Official Video)","duration":285,"channel":"Massive Attack","uploader":"Massive Attack","webpage_url":"https://www.youtube.com/watch?v=u7K72X4eo_s","availability":null}"#;
        let parsed: YtDlpEntry = serde_json::from_str(line).expect("parse flat entry");
        let track = parsed.into_track().expect("track");
        assert_eq!(track.id, "u7K72X4eo_s");
        assert_eq!(track.artist, "Massive Attack");
        assert_eq!(track.duration_seconds, Some(285));
    }
}
