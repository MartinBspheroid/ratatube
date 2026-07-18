//! yt-dlp subprocess client.
//!
//! yt-dlp processes are short-lived per operation (PRD section 14). All
//! invocations use direct argument execution without a shell (PRD 19).

use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

use crate::error::{AppError, Result};
use crate::media::Track;

/// Default timeout for a metadata operation.
const METADATA_TIMEOUT: Duration = Duration::from_secs(60);
const STDOUT_LIMIT: usize = 16 * 1024 * 1024;
const STDERR_LIMIT: usize = 1024 * 1024;

/// Raw yt-dlp JSON shape for a single entry (subset of fields we use).
#[derive(Debug, Deserialize)]
struct YtDlpEntry {
    id: Option<String>,
    title: Option<String>,
    /// yt-dlp emits both `uploader` and `channel`; coalesce on use.
    uploader: Option<String>,
    channel: Option<String>,
    channel_id: Option<String>,
    channel_url: Option<String>,
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
    acodec: Option<String>,
    abr: Option<f64>,
    asr: Option<u32>,
    audio_channels: Option<u8>,
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
    /// Normalize full-video metadata into the application detail model.
    fn into_details(self) -> crate::media::TrackDetails {
        let mut chapters: Vec<crate::media::Chapter> = self
            .chapters
            .unwrap_or_default()
            .into_iter()
            .filter_map(|chapter| {
                Some(crate::media::Chapter {
                    start_seconds: chapter.start_time?,
                    title: chapter.title.unwrap_or_default(),
                })
            })
            .collect();
        if chapters.is_empty()
            && let Some(description) = self.description.as_deref()
        {
            chapters = crate::media::parse_chapters_from_description(description);
        }
        crate::media::TrackDetails {
            description: self.description,
            view_count: self.view_count,
            like_count: self.like_count,
            upload_date: self.upload_date,
            uploader: self.uploader.or(self.channel),
            categories: self.categories.into_iter().flatten().collect(),
            acodec: self.acodec,
            abr: self.abr,
            asr: self.asr,
            audio_channels: self.audio_channels,
            chapters,
        }
    }

    /// Normalize into a [`Track`] while preserving a concrete rejection reason.
    fn into_track(self) -> std::result::Result<Track, SkipReason> {
        let id = self.id.ok_or(SkipReason::MissingId)?;
        let title = self.title.ok_or(SkipReason::MissingTitle)?;
        if title == "[Deleted video]" {
            return Err(SkipReason::Deleted);
        }
        if title == "[Private video]" {
            return Err(SkipReason::Private);
        }
        let availability = match self.availability.as_deref() {
            Some("private") => return Err(SkipReason::Private),
            Some("unavailable") => return Err(SkipReason::Unavailable),
            Some(_) => crate::media::Availability::Available,
            None => crate::media::Availability::Unknown,
        };
        Ok(Track {
            webpage_url: self
                .webpage_url
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}")),
            id,
            title,
            artist: self
                .uploader
                .or(self.channel)
                .unwrap_or_else(|| "Unknown".to_string()),
            channel_id: self.channel_id,
            channel_url: self.channel_url,
            duration_seconds: self.duration.map(|d| d.max(0.0) as u64),
            thumbnail_url: self.thumbnail,
            availability,
        })
    }
}

/// Exact reason an external playlist entry was not importable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    MissingId,
    MissingTitle,
    Deleted,
    Private,
    Unavailable,
}

/// Traceable rejection counts from yt-dlp normalization.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportRejections {
    pub missing_id: usize,
    pub missing_title: usize,
    pub deleted: usize,
    pub private: usize,
    pub unavailable: usize,
}

impl ImportRejections {
    fn record(&mut self, reason: SkipReason) {
        match reason {
            SkipReason::MissingId => self.missing_id += 1,
            SkipReason::MissingTitle => self.missing_title += 1,
            SkipReason::Deleted => self.deleted += 1,
            SkipReason::Private => self.private += 1,
            SkipReason::Unavailable => self.unavailable += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.missing_id + self.missing_title + self.deleted + self.private + self.unavailable
    }
}

/// Result of a flat playlist fetch, including skipped entries (PRD 10.8).
#[derive(Debug)]
pub struct PlaylistFetch {
    pub title: String,
    pub remote_id: Option<String>,
    pub tracks: Vec<Track>,
    /// Entries that could not be normalized (deleted, private, missing data).
    pub rejections: ImportRejections,
}

/// Async client wrapping the yt-dlp executable.
#[derive(Debug, Clone)]
pub struct YtDlp {
    binary: String,
    stdout_limit: usize,
    stderr_limit: usize,
}

impl YtDlp {
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            stdout_limit: STDOUT_LIMIT,
            stderr_limit: STDERR_LIMIT,
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
        // --dump-json emits one JSON object per line.
        for line in output.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str::<YtDlpEntry>(line) {
                Ok(entry) => {
                    if let Ok(track) = entry.into_track() {
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
        let output = self.run(&["-J", "--no-download", "--", url]).await?;
        let entry: YtDlpEntry = serde_json::from_str(&output)?;
        entry
            .into_track()
            .map_err(|reason| AppError::Resolve(format!("{reason:?} for {url}")))
    }

    /// Fetch extended metadata (description, stats, chapters) for the
    /// detail panel. When the uploader set no chapters, a tracklist parsed
    /// from the description fills in (DJ mixes).
    pub async fn fetch_details(&self, url: &str) -> Result<crate::media::TrackDetails> {
        let output = self.run(&["-J", "--no-download", "--", url]).await?;
        let entry: YtDlpEntry = serde_json::from_str(&output)?;
        Ok(entry.into_details())
    }

    /// Fetch all entries of a playlist URL, metadata only.
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
            title: root.title.clone().unwrap_or_else(|| "Playlist".to_string()),
            remote_id: root.id.clone(),
            rejections,
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
        let mut command = Command::new(&self.binary);
        command
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            // Prevent orphan processes (PRD section 14).
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppError::MissingDependency(self.binary.clone())
            } else {
                AppError::Process {
                    command: self.binary.clone(),
                    message: err.to_string(),
                }
            }
        })?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let operation = async {
            let (stdout, stderr) = tokio::try_join!(
                read_limited(stdout, self.stdout_limit, "yt-dlp stdout"),
                read_limited(stderr, self.stderr_limit, "yt-dlp stderr")
            )?;
            let status = child.wait().await?;
            Ok::<_, AppError>((status, stdout, stderr))
        };
        match tokio::time::timeout(METADATA_TIMEOUT, operation).await {
            Ok(Ok((status, stdout, _))) if status.success() => {
                Ok(String::from_utf8_lossy(&stdout).into_owned())
            }
            Ok(Ok((_, _, stderr))) => Err(AppError::YtDlp(redact_urls(
                String::from_utf8_lossy(&stderr).trim(),
            ))),
            Ok(Err(err)) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                Err(err)
            }
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                Err(AppError::Timeout(format!(
                    "yt-dlp exceeded {}s",
                    METADATA_TIMEOUT.as_secs()
                )))
            }
        }
    }
}

async fn read_limited(
    mut reader: impl AsyncRead + Unpin,
    limit: usize,
    resource: &str,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            return Ok(bytes);
        }
        if bytes.len() + read > limit {
            return Err(AppError::ResourceLimit {
                resource: resource.to_string(),
                limit,
            });
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}

fn redact_urls(message: &str) -> String {
    message
        .split_whitespace()
        .map(|part| {
            if part.starts_with("http://") || part.starts_with("https://") {
                "[url redacted]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod parse_tests {
    use super::{YtDlpEntry, read_limited, redact_urls};

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

    #[test]
    fn parses_stable_channel_identity() {
        let entry: YtDlpEntry = serde_json::from_str(
            r#"{"id":"v","title":"Video","channel":"Channel","channel_id":"UC123","channel_url":"https://www.youtube.com/channel/UC123"}"#,
        )
        .expect("entry");
        let track = entry.into_track().expect("track");
        assert_eq!(track.channel_id.as_deref(), Some("UC123"));
        assert_eq!(
            track.channel_url.as_deref(),
            Some("https://www.youtube.com/channel/UC123")
        );
    }

    #[test]
    fn parses_audio_format_fields_into_track_details() {
        let line = r#"{"id":"mix","title":"Set","acodec":"opus","abr":128.5,"asr":48000,"audio_channels":2}"#;
        let details = serde_json::from_str::<YtDlpEntry>(line)
            .expect("parse details")
            .into_details();
        assert_eq!(details.acodec.as_deref(), Some("opus"));
        assert_eq!(details.abr, Some(128.5));
        assert_eq!(details.asr, Some(48_000));
        assert_eq!(details.audio_channels, Some(2));
    }

    #[tokio::test]
    async fn bounded_reader_accepts_exact_limit_and_rejects_one_byte_over() {
        let exact = read_limited(&b"1234"[..], 4, "test")
            .await
            .expect("exact limit");
        assert_eq!(exact, b"1234");
        assert!(read_limited(&b"12345"[..], 4, "test").await.is_err());
    }

    #[test]
    fn stderr_redaction_removes_signed_urls() {
        let message = redact_urls("failed https://googlevideo.example/x?sig=SECRET retry");
        assert_eq!(message, "failed [url redacted] retry");
        assert!(!message.contains("SECRET"));
    }
}
