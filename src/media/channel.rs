//! Bounded channel-video page requests and normalization.

use std::collections::HashSet;

use crate::error::{AppError, Result};
use crate::media::Track;
use crate::media::yt_dlp::{ImportRejections, YtDlpEntry};

/// Number of source entries requested for one channel page.
pub const CHANNEL_PAGE_SIZE: usize = 30;

/// A zero-based request for a bounded channel-video page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelPageRequest {
    /// Trusted YouTube channel URL, without query or fragment.
    pub channel_url: String,
    /// Zero-based page number.
    pub page: usize,
}

/// A normalized page of public channel videos.
#[derive(Debug, PartialEq)]
pub struct ChannelPage {
    /// Unique playable tracks in source (newest-first) order.
    pub tracks: Vec<Track>,
    /// Source rows that could not be normalized.
    pub rejections: ImportRejections,
    /// Whether yt-dlp returned fewer source rows than requested.
    pub exhausted: bool,
}

impl ChannelPageRequest {
    /// Return one-based inclusive yt-dlp playlist bounds.
    pub fn bounds(&self) -> Result<(usize, usize)> {
        let start = self
            .page
            .checked_mul(CHANNEL_PAGE_SIZE)
            .and_then(|offset| offset.checked_add(1))
            .ok_or_else(|| AppError::InvalidUrl("channel page is too large".into()))?;
        let end = start
            .checked_add(CHANNEL_PAGE_SIZE - 1)
            .ok_or_else(|| AppError::InvalidUrl("channel page is too large".into()))?;
        Ok((start, end))
    }

    /// Normalize the channel URL to its single trailing `/videos` surface.
    pub fn videos_url(&self) -> Result<String> {
        normalize_channel_url(&self.channel_url)
    }
}

pub(super) fn parse_channel_page(output: &str) -> ChannelPage {
    let lines: Vec<_> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let mut tracks = Vec::new();
    let mut seen = HashSet::new();
    let mut rejections = ImportRejections::default();
    for line in &lines {
        match serde_json::from_str::<YtDlpEntry>(line) {
            Ok(entry) => match entry.into_track() {
                Ok(track) if seen.insert(track.id.clone()) => tracks.push(track),
                Ok(_) => {}
                Err(reason) => rejections.record(reason),
            },
            Err(_) => rejections.record_malformed(),
        }
    }
    ChannelPage {
        tracks,
        rejections,
        exhausted: lines.len() < CHANNEL_PAGE_SIZE,
    }
}

fn normalize_channel_url(raw: &str) -> Result<String> {
    if raw.contains(['?', '#']) {
        return Err(AppError::InvalidUrl(raw.to_string()));
    }
    let rest = raw
        .strip_prefix("https://")
        .ok_or_else(|| AppError::InvalidUrl(raw.to_string()))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if !matches!(
        host.to_ascii_lowercase().as_str(),
        "youtube.com" | "www.youtube.com"
    ) {
        return Err(AppError::InvalidUrl(raw.to_string()));
    }
    let mut segments: Vec<_> = path.split('/').filter(|part| !part.is_empty()).collect();
    if segments.last() == Some(&"videos") {
        segments.pop();
    }
    let valid = match segments.as_slice() {
        [handle] => handle.starts_with('@') && handle.len() > 1,
        [kind, value] => matches!(*kind, "channel" | "c" | "user") && !value.is_empty(),
        _ => false,
    };
    if !valid {
        return Err(AppError::InvalidUrl(raw.to_string()));
    }
    Ok(format!(
        "https://www.youtube.com/{}/videos",
        segments.join("/")
    ))
}

#[cfg(test)]
mod tests {
    use super::{ChannelPageRequest, normalize_channel_url, parse_channel_page};

    #[test]
    fn channel_page_bounds_are_inclusive_and_one_based() {
        let request = |page| ChannelPageRequest {
            channel_url: "https://youtube.com/@x".into(),
            page,
        };
        assert_eq!(request(0).bounds().expect("page 0"), (1, 30));
        assert_eq!(request(1).bounds().expect("page 1"), (31, 60));
    }

    #[test]
    fn channel_urls_normalize_to_one_videos_suffix() {
        assert_eq!(
            normalize_channel_url("https://youtube.com/@artist/").expect("handle"),
            "https://www.youtube.com/@artist/videos"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/channel/UC1/videos").expect("channel"),
            "https://www.youtube.com/channel/UC1/videos"
        );
    }

    #[test]
    fn unsafe_or_non_channel_urls_are_rejected() {
        for url in [
            "http://youtube.com/@x",
            "https://evil-youtube.com/@x",
            "https://youtube.com/watch",
            "https://youtube.com/@x?view=1",
            "https://youtube.com/@x#videos",
        ] {
            assert!(normalize_channel_url(url).is_err(), "accepted {url}");
        }
    }

    #[test]
    fn ndjson_keeps_order_deduplicates_and_counts_rejections() {
        let page = parse_channel_page(
            "{\"id\":\"new\",\"title\":\"Newest\"}\nnot-json\n{\"id\":\"private\",\"title\":\"[Private video]\"}\n{\"id\":\"new\",\"title\":\"Duplicate\"}\n{\"id\":\"old\",\"title\":\"Older\"}\n",
        );
        assert_eq!(
            page.tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            ["new", "old"]
        );
        assert_eq!(page.rejections.malformed, 1);
        assert_eq!(page.rejections.private, 1);
        assert!(page.exhausted);
    }

    #[test]
    fn exhaustion_uses_source_line_count_not_accepted_tracks() {
        let output = (0..30)
            .map(|index| format!("{{\"id\":\"{index}\",\"title\":\"[Private video]\"}}"))
            .collect::<Vec<_>>()
            .join("\n");
        let page = parse_channel_page(&output);
        assert!(page.tracks.is_empty());
        assert_eq!(page.rejections.private, 30);
        assert!(!page.exhausted);
    }
}
