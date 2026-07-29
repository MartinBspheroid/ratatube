use serde::Deserialize;

use super::SkipReason;
use crate::media::{Availability, Chapter, Track, TrackDetails, parse_chapters_from_description};

/// Raw yt-dlp JSON shape for a single entry (subset of fields we use).
#[derive(Debug, Deserialize)]
pub struct YtDlpEntry {
    pub id: Option<String>,
    pub title: Option<String>,
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
    pub entries: Vec<YtDlpEntry>,
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
    pub fn into_details(self) -> TrackDetails {
        let mut chapters: Vec<Chapter> = self
            .chapters
            .unwrap_or_default()
            .into_iter()
            .filter_map(|chapter| {
                Some(Chapter {
                    start_seconds: chapter.start_time?,
                    title: chapter.title.unwrap_or_default(),
                })
            })
            .collect();
        if chapters.is_empty()
            && let Some(description) = self.description.as_deref()
        {
            chapters = parse_chapters_from_description(description);
        }
        TrackDetails {
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
    pub fn into_track(self) -> std::result::Result<Track, SkipReason> {
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
            Some(_) => Availability::Available,
            None => Availability::Unknown,
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
            duration_seconds: self.duration.map(|duration| duration.max(0.0) as u64),
            thumbnail_url: self.thumbnail,
            availability,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::YtDlpEntry;

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
}
