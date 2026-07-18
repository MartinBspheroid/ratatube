//! Media discovery, metadata, and stream resolution via yt-dlp.

pub mod import;
pub mod resolver;
pub mod search;
pub mod yt_dlp;

use serde::{Deserialize, Serialize};

/// Decode artwork under strict dimensions and allocation limits.
pub fn decode_thumbnail(bytes: &[u8]) -> image::ImageResult<image::DynamicImage> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader.decode()
}

/// A chapter within a video: yt-dlp chapters when the uploader set them,
/// or timestamps parsed from a tracklist in the description (DJ mixes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub title: String,
    pub start_seconds: f64,
}

/// Index of the chapter containing `position_seconds`, if any.
pub fn chapter_at(chapters: &[Chapter], position_seconds: f64) -> Option<usize> {
    chapters
        .iter()
        .rposition(|c| c.start_seconds <= position_seconds)
}

/// Parse a tracklist with timestamps out of a video description.
///
/// Accepts lines containing an `H:MM:SS` or `M:SS` token; the rest of the
/// line becomes the title. Only a list of two or more entries with
/// non-decreasing timestamps counts — anything else is noise, not a
/// tracklist.
pub fn parse_chapters_from_description(description: &str) -> Vec<Chapter> {
    fn parse_hms(token: &str) -> Option<f64> {
        let parts: Vec<&str> = token.split(':').collect();
        if !(2..=3).contains(&parts.len()) {
            return None;
        }
        let mut seconds = 0u64;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() || part.len() > 2 || !part.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let value: u64 = part.parse().ok()?;
            // Trailing fields are base-60 digits.
            if i > 0 && parts.len() == 3 && value >= 60 {
                return None;
            }
            if i + 1 == parts.len() && value >= 60 {
                return None;
            }
            seconds = seconds * 60 + value;
        }
        Some(seconds as f64)
    }

    let mut chapters = Vec::new();
    for line in description.lines() {
        let mut timestamp = None;
        let mut title_tokens: Vec<&str> = Vec::new();
        for token in line.split_whitespace() {
            let cleaned = token.trim_matches(|c: char| !c.is_ascii_digit() && c != ':');
            match parse_hms(cleaned) {
                // First timestamp wins; later ones (e.g. end times) are noise.
                Some(seconds) if timestamp.is_none() => timestamp = Some(seconds),
                Some(_) => {}
                None => title_tokens.push(token),
            }
        }
        if let Some(start_seconds) = timestamp {
            let title = title_tokens
                .join(" ")
                .trim_matches(|c: char| {
                    c.is_whitespace()
                        || matches!(
                            c,
                            '-' | '–' | '—' | '|' | ':' | '.' | '(' | ')' | '[' | ']' | '*'
                        )
                })
                .to_string();
            chapters.push(Chapter {
                title: if title.is_empty() {
                    format!("Chapter {}", chapters.len() + 1)
                } else {
                    title
                },
                start_seconds,
            });
        }
    }
    let increasing = chapters
        .windows(2)
        .all(|w| w[1].start_seconds >= w[0].start_seconds);
    if chapters.len() >= 2 && increasing {
        chapters
    } else {
        Vec::new()
    }
}

/// Extended metadata for a single video, fetched on demand for the
/// now-playing view and detail panels (PRD 10.1 metadata detail).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackDetails {
    pub description: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    /// yt-dlp `upload_date` as YYYYMMDD.
    pub upload_date: Option<String>,
    pub uploader: Option<String>,
    pub categories: Vec<String>,
    /// Selected audio codec reported by yt-dlp, for example `opus`.
    pub acodec: Option<String>,
    /// Approximate audio bitrate in kilobits per second.
    pub abr: Option<f64>,
    /// Audio sample rate in hertz.
    pub asr: Option<u32>,
    /// Number of audio channels in the selected format.
    pub audio_channels: Option<u8>,
    /// Uploader-set chapters, or a tracklist parsed from the description.
    pub chapters: Vec<Chapter>,
}

impl TrackDetails {
    /// Upload date formatted as YYYY-MM-DD for display.
    pub fn formatted_upload_date(&self) -> Option<String> {
        let raw = self.upload_date.as_deref()?;
        chrono::NaiveDate::parse_from_str(raw, "%Y%m%d")
            .ok()
            .map(|date| date.format("%Y-%m-%d").to_string())
    }
}

/// Format a large count compactly, e.g. 12_345_678 -> "12.3M".
pub fn format_count(count: u64) -> String {
    const MILLION: u64 = 1_000_000;
    const THOUSAND: u64 = 1_000;
    if count >= MILLION {
        let tenths = count / (MILLION / 10);
        format!("{}.{:01}M", tenths / 10, tenths % 10)
    } else if count >= THOUSAND {
        let tenths = count / (THOUSAND / 10);
        format!("{}.{:01}K", tenths / 10, tenths % 10)
    } else {
        count.to_string()
    }
}

/// Availability of a track as far as the library knows (PRD 10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    #[default]
    Available,
    Unavailable,
    Private,
    Unknown,
}

/// A single playable item.
///
/// The canonical YouTube page URL is stored permanently; resolved stream
/// URLs are runtime-only state and are never persisted (PRD 10.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// YouTube video ID.
    pub id: String,
    pub title: String,
    /// Channel or artist name.
    pub artist: String,
    /// Stable YouTube channel identifier, when available.
    #[serde(default)]
    pub channel_id: Option<String>,
    /// Canonical YouTube channel URL, when available.
    #[serde(default)]
    pub channel_url: Option<String>,
    pub webpage_url: String,
    pub duration_seconds: Option<u64>,
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub availability: Availability,
}

impl Track {
    /// Construct a track with unknown availability and duration.
    pub fn new(id: impl Into<String>, title: impl Into<String>, artist: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            webpage_url: format!("https://www.youtube.com/watch?v={id}"),
            id,
            title: title.into(),
            artist: artist.into(),
            channel_id: None,
            channel_url: None,
            duration_seconds: None,
            thumbnail_url: None,
            availability: Availability::Unknown,
        }
    }
}

#[cfg(test)]
mod chapter_tests {
    use super::*;

    #[test]
    fn parses_tracklist_with_timestamps() {
        let desc = "Tracklist:\n0:00 Intro\n3:45 Artist - Song One\n1:02:11 Artist Two - Closer\nthanks for listening";
        let chapters = parse_chapters_from_description(desc);
        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Intro");
        assert_eq!(chapters[1].start_seconds, 225.0);
        assert_eq!(chapters[2].start_seconds, 3731.0);
        assert_eq!(chapters[2].title, "Artist Two - Closer");
    }

    #[test]
    fn ignores_descriptions_without_a_tracklist() {
        assert!(parse_chapters_from_description("check my mix at 3:45 somewhere").is_empty());
        assert!(parse_chapters_from_description("no timestamps here at all").is_empty());
    }

    #[test]
    fn rejects_decreasing_timestamps() {
        let desc = "5:00 Later\n0:30 Earlier";
        assert!(parse_chapters_from_description(desc).is_empty());
    }

    #[test]
    fn chapter_at_finds_current() {
        let chapters = vec![
            Chapter {
                title: "a".into(),
                start_seconds: 0.0,
            },
            Chapter {
                title: "b".into(),
                start_seconds: 100.0,
            },
            Chapter {
                title: "c".into(),
                start_seconds: 200.0,
            },
        ];
        assert_eq!(chapter_at(&chapters, 0.0), Some(0));
        assert_eq!(chapter_at(&chapters, 150.0), Some(1));
        assert_eq!(chapter_at(&chapters, 999.0), Some(2));
        assert_eq!(chapter_at(&[], 10.0), None);
    }

    #[test]
    fn thumbnail_decoder_rejects_dimensions_above_limit() {
        let image = image::DynamicImage::new_rgba8(4097, 1);
        let mut encoded = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("encode png");

        assert!(decode_thumbnail(encoded.get_ref()).is_err());
    }

    #[test]
    fn upload_date_requires_a_real_calendar_date() {
        let valid = TrackDetails {
            upload_date: Some("20240229".to_string()),
            ..TrackDetails::default()
        };
        assert_eq!(valid.formatted_upload_date().as_deref(), Some("2024-02-29"));

        for raw in ["20230229", "20241301", "not-a-date", "1234567"] {
            let details = TrackDetails {
                upload_date: Some(raw.to_string()),
                ..TrackDetails::default()
            };
            assert_eq!(details.formatted_upload_date(), None, "raw: {raw}");
        }
    }

    #[test]
    fn compact_count_boundaries_are_stable() {
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1.0K");
        assert_eq!(format_count(999_999), "999.9K");
        assert_eq!(format_count(1_000_000), "1.0M");
    }
}
