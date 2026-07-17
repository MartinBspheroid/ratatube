//! Configuration model matching PRD section 11.3.

use serde::{Deserialize, Serialize};

/// Current configuration schema version.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Nerd Font icon selection mode (PRD section 10.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconMode {
    /// Conservative default; may be overridden by the user.
    #[default]
    Auto,
    /// Force Nerd Font icons.
    NerdFont,
    /// Force plain ASCII fallbacks.
    Ascii,
}

/// Root configuration document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub schema_version: u32,
    pub playback: PlaybackConfig,
    pub search: SearchConfig,
    pub history: HistoryConfig,
    pub ui: UiConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlaybackConfig {
    pub default_volume: u8,
    pub audio_only: bool,
    /// Continue to the next queue item when a track fails (PRD 10.4).
    pub continue_on_error: bool,
    /// Resolve the stream URL shortly before playback (PRD 10.4).
    pub resolve_before_playback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchConfig {
    pub result_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryConfig {
    pub enabled: bool,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiConfig {
    pub icons: IconMode,
    pub show_footer_hints: bool,
    /// Progress refresh interval in milliseconds (PRD 10.3).
    pub progress_refresh_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PathsConfig {
    /// Executable name or absolute path for mpv.
    pub mpv: String,
    /// Executable name or absolute path for yt-dlp.
    pub yt_dlp: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            playback: PlaybackConfig::default(),
            search: SearchConfig::default(),
            history: HistoryConfig::default(),
            ui: UiConfig::default(),
            paths: PathsConfig::default(),
        }
    }
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            default_volume: 70,
            audio_only: true,
            continue_on_error: true,
            resolve_before_playback: true,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { result_limit: 20 }
    }
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 500,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            icons: IconMode::Auto,
            show_footer_hints: true,
            progress_refresh_ms: 500,
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            mpv: "mpv".to_string(),
            yt_dlp: "yt-dlp".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).expect("serialize");
        let parsed: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(parsed.playback.default_volume, 70);
        assert_eq!(parsed.search.result_limit, 20);
        assert_eq!(parsed.history.max_entries, 500);
    }

    #[test]
    fn partial_config_uses_defaults() {
        let parsed: Config = serde_json::from_str(r#"{"search": {"resultLimit": 50}}"#)
            .expect("deserialize partial");
        assert_eq!(parsed.search.result_limit, 50);
        assert_eq!(parsed.playback.default_volume, 70);
    }
}
