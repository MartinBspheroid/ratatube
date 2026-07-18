use serde::{Deserialize, Serialize};

use super::Chapter;

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

#[cfg(test)]
mod tests {
    use super::{TrackDetails, format_count};

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
