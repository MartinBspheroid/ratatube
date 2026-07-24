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

impl IconMode {
    /// Every mode, in settings-menu cycling order.
    pub const ALL: [IconMode; 3] = [IconMode::Auto, IconMode::NerdFont, IconMode::Ascii];

    /// The configuration spelling, shown in the settings menu.
    pub fn label(self) -> &'static str {
        match self {
            IconMode::Auto => "auto",
            IconMode::NerdFont => "nerd-font",
            IconMode::Ascii => "ascii",
        }
    }
}

/// Built-in color theme selected in `ui.theme` and the ctrl+p settings menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    /// The original ratatube palette: cyan and magenta on deep navy.
    #[default]
    Neon,
    /// Catppuccin Mocha (catppuccin.com).
    CatppuccinMocha,
    /// Solarized Dark (ethanschoonover.com/solarized).
    SolarizedDark,
    /// Tokyo Night (github.com/enkia/tokyo-night-vscode-theme).
    TokyoNight,
    /// Gruvbox dark mode (github.com/morhetz/gruvbox).
    GruvboxDark,
    /// Nord (nordtheme.com).
    Nord,
    /// Dracula (draculatheme.com).
    Dracula,
}

impl ThemeName {
    /// Every selectable theme, in settings-menu order.
    pub const ALL: [ThemeName; 7] = [
        ThemeName::Neon,
        ThemeName::CatppuccinMocha,
        ThemeName::SolarizedDark,
        ThemeName::TokyoNight,
        ThemeName::GruvboxDark,
        ThemeName::Nord,
        ThemeName::Dracula,
    ];

    /// Human-readable name shown in the settings menu.
    pub fn label(self) -> &'static str {
        match self {
            ThemeName::Neon => "Neon",
            ThemeName::CatppuccinMocha => "Catppuccin Mocha",
            ThemeName::SolarizedDark => "Solarized Dark",
            ThemeName::TokyoNight => "Tokyo Night",
            ThemeName::GruvboxDark => "Gruvbox Dark",
            ThemeName::Nord => "Nord",
            ThemeName::Dracula => "Dracula",
        }
    }
}

/// What to do with the previous session's track on launch (PRD-next:
/// instant play). `Paused` preloads the last track at its saved position so
/// a single Space resumes it; `Playing` starts audio immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResumeMode {
    /// Do not restore the previous session.
    Off,
    /// Preload the last track paused at its saved position (default).
    #[default]
    Paused,
    /// Restore and start playing immediately.
    Playing,
}

impl ResumeMode {
    /// Every mode, in settings-menu cycling order.
    pub const ALL: [ResumeMode; 3] = [ResumeMode::Off, ResumeMode::Paused, ResumeMode::Playing];

    /// The configuration spelling, shown in the settings menu.
    pub fn label(self) -> &'static str {
        match self {
            ResumeMode::Off => "off",
            ResumeMode::Paused => "paused",
            ResumeMode::Playing => "playing",
        }
    }
}

/// Root configuration document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub playback: PlaybackConfig,
    pub search: SearchConfig,
    pub history: HistoryConfig,
    pub ui: UiConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct PlaybackConfig {
    pub default_volume: u8,
    /// Continue to the next queue item when a track fails (PRD 10.4).
    pub continue_on_error: bool,
    /// Session restore behavior on launch.
    pub resume_on_launch: ResumeMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SearchConfig {
    pub result_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct HistoryConfig {
    pub enabled: bool,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct UiConfig {
    pub icons: IconMode,
    /// Progress refresh interval in milliseconds (PRD 10.3).
    pub progress_refresh_ms: u64,
    /// Selected color theme; also switchable at runtime with ctrl+p.
    pub theme: ThemeName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
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
            continue_on_error: true,
            resume_on_launch: ResumeMode::default(),
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
            progress_refresh_ms: 500,
            theme: ThemeName::default(),
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

    #[test]
    fn rejects_removed_inert_configuration_keys() {
        for json in [
            r#"{"playback":{"audioOnly":false}}"#,
            r#"{"playback":{"resolveBeforePlayback":false}}"#,
            r#"{"ui":{"showFooterHints":false}}"#,
        ] {
            assert!(
                serde_json::from_str::<Config>(json).is_err(),
                "json: {json}"
            );
        }
    }

    #[test]
    fn theme_names_serialize_as_kebab_case() {
        let parsed: Config = serde_json::from_str(r#"{"ui": {"theme": "catppuccin-mocha"}}"#)
            .expect("deserialize theme");
        assert_eq!(parsed.ui.theme, ThemeName::CatppuccinMocha);
        let json = serde_json::to_string(&Config::default()).expect("serialize");
        assert!(json.contains(r#""theme":"neon""#), "{json}");
    }

    #[test]
    fn checked_in_example_is_valid_and_current() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.json");
        let json = std::fs::read_to_string(path).expect("read config example");
        let parsed: Config = serde_json::from_str(&json).expect("parse config example");
        assert_eq!(parsed.schema_version, CONFIG_SCHEMA_VERSION);
    }
}
