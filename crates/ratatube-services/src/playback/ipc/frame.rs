use serde::Deserialize;
use serde_json::Value;

use crate::playback::events::{AudioLevels, PlaybackEvent};

/// The mpv property carrying the level-meter filter's metadata.
pub(super) const LEVELS_PROPERTY: &str = "af-metadata/vis";

/// Raw event frame from mpv.
#[derive(Debug, Deserialize)]
pub(super) struct MpvFrame {
    #[serde(default)]
    pub(super) request_id: Option<u64>,
    #[serde(default)]
    pub(super) event: Option<String>,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) data: Option<Value>,
    #[serde(default)]
    pub(super) reason: Option<String>,
    #[serde(default)]
    pub(super) error: Option<String>,
}

/// Translate one raw mpv frame into a typed event.
#[cfg(test)]
pub(super) fn parse_frame(line: &str) -> Option<PlaybackEvent> {
    let frame: MpvFrame = serde_json::from_str(line).ok()?;
    event_from_frame(frame)
}

pub(super) fn event_from_frame(frame: MpvFrame) -> Option<PlaybackEvent> {
    if let Some(error) = &frame.error
        && error != "success"
    {
        return Some(PlaybackEvent::PlaybackError(error.clone()));
    }
    match frame.event.as_deref()? {
        "playback-restart" => Some(PlaybackEvent::Started),
        "file-loaded" => Some(PlaybackEvent::FileLoaded),
        "end-file" => Some(PlaybackEvent::EndFile {
            reason: frame.reason.unwrap_or_else(|| "unknown".to_string()),
        }),
        "property-change" => property_event(frame.name.as_deref(), frame.data),
        _ => None,
    }
}

fn property_event(name: Option<&str>, data: Option<Value>) -> Option<PlaybackEvent> {
    match (name, data) {
        (Some("time-pos"), Some(Value::Number(number))) => {
            number.as_f64().map(PlaybackEvent::PositionChanged)
        }
        (Some("duration"), Some(Value::Number(number))) => {
            number.as_f64().map(PlaybackEvent::DurationChanged)
        }
        (Some("pause"), Some(Value::Bool(value))) => Some(PlaybackEvent::PauseChanged(value)),
        (Some("volume"), Some(Value::Number(number))) => {
            number.as_f64().map(PlaybackEvent::VolumeChanged)
        }
        (Some("mute"), Some(Value::Bool(value))) => Some(PlaybackEvent::MuteChanged(value)),
        (Some("speed"), Some(Value::Number(number))) => {
            number.as_f64().map(PlaybackEvent::SpeedChanged)
        }
        (Some(LEVELS_PROPERTY), Some(Value::Object(metadata))) => {
            audio_levels(&metadata).map(PlaybackEvent::AudioLevels)
        }
        _ => None,
    }
}

/// Parse the astats metadata map (string values in dBFS / rates) into typed
/// levels. Silence reports "-inf"; clamp it to the meter floor.
fn audio_levels(metadata: &serde_json::Map<String, Value>) -> Option<AudioLevels> {
    let metric = |key: &str| -> Option<f32> {
        metadata
            .get(key)?
            .as_str()?
            .parse::<f32>()
            .ok()
            .map(|value| value.clamp(-90.0, 20.0))
    };
    let channel_mean = |suffix: &str| -> Option<f32> {
        let values: Vec<f32> = (1..=2)
            .filter_map(|channel| metric(&format!("lavfi.astats.{channel}.{suffix}")))
            .collect();
        (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
    };
    let channel_max = |suffix: &str| -> Option<f32> {
        (1..=2)
            .filter_map(|channel| metric(&format!("lavfi.astats.{channel}.{suffix}")))
            .reduce(f32::max)
    };
    Some(AudioLevels {
        rms_db: metric("lavfi.astats.Overall.RMS_level").or(channel_mean("RMS_level"))?,
        peak_db: channel_max("Peak_level")?,
        zcr: channel_mean("Zero_crossings_rate")?.max(0.0),
    })
}
