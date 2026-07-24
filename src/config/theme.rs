//! Theme identity: sixteen families, each with a dark and a light variant.
//!
//! `ThemeName` is the flat configuration value (`ui.theme`); `ThemeFamily`
//! and `ThemeMode` are the two axes the settings menu navigates. The variant
//! table is the single source of truth tying the three together.

use serde::{Deserialize, Serialize};

/// Built-in color theme selected in `ui.theme` and the ctrl+p settings menu.
/// Each name is one variant of a [`ThemeFamily`] in one [`ThemeMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    /// The original ratatube palette: cyan and magenta on deep navy.
    #[default]
    Neon,
    /// Neon on paper: the same cyan/magenta identity, darkened for light.
    NeonLight,
    /// Catppuccin Mocha (catppuccin.com).
    CatppuccinMocha,
    /// Catppuccin Latte (catppuccin.com).
    CatppuccinLatte,
    /// Solarized Dark (ethanschoonover.com/solarized).
    SolarizedDark,
    /// Solarized Light (ethanschoonover.com/solarized).
    SolarizedLight,
    /// Tokyo Night (github.com/enkia/tokyo-night-vscode-theme).
    TokyoNight,
    /// Tokyo Night Day (github.com/folke/tokyonight.nvim).
    TokyoNightDay,
    /// Gruvbox dark mode (github.com/morhetz/gruvbox).
    GruvboxDark,
    /// Gruvbox light mode (github.com/morhetz/gruvbox).
    GruvboxLight,
    /// Nord on Polar Night (nordtheme.com).
    Nord,
    /// Nord on Snow Storm (nordtheme.com).
    NordLight,
    /// Dracula (draculatheme.com/spec).
    Dracula,
    /// Alucard, Dracula's official light theme (draculatheme.com/spec).
    Alucard,
    /// Atom One Dark (github.com/atom/one-dark-syntax).
    OneDark,
    /// Atom One Light (github.com/atom/one-light-syntax).
    OneLight,
    /// Rosé Pine main variant (rosepinetheme.com).
    RosePine,
    /// Rosé Pine Dawn (rosepinetheme.com).
    RosePineDawn,
    /// Kanagawa Wave (github.com/rebelot/kanagawa.nvim).
    KanagawaWave,
    /// Kanagawa Lotus (github.com/rebelot/kanagawa.nvim).
    KanagawaLotus,
    /// Everforest dark, medium contrast (github.com/sainnhe/everforest).
    EverforestDark,
    /// Everforest light, medium contrast (github.com/sainnhe/everforest).
    EverforestLight,
    /// Ayu Dark (github.com/ayu-theme).
    AyuDark,
    /// Ayu Light (github.com/ayu-theme).
    AyuLight,
    /// Night Owl (github.com/sdras/night-owl-vscode-theme).
    NightOwl,
    /// Light Owl (github.com/sdras/night-owl-vscode-theme).
    LightOwl,
    /// GitHub Dark, Primer colors (github.com/primer/github-vscode-theme).
    GithubDark,
    /// GitHub Light, Primer colors (github.com/primer/github-vscode-theme).
    GithubLight,
    /// Selenized dark (github.com/jan-warchol/selenized).
    SelenizedDark,
    /// Selenized light (github.com/jan-warchol/selenized).
    SelenizedLight,
    /// Flexoki dark (stephango.com/flexoki).
    FlexokiDark,
    /// Flexoki light (stephango.com/flexoki).
    FlexokiLight,
}

/// One of the sixteen theme families the settings menu lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeFamily {
    Neon,
    Catppuccin,
    Solarized,
    TokyoNight,
    Gruvbox,
    Nord,
    Dracula,
    One,
    RosePine,
    Kanagawa,
    Everforest,
    Ayu,
    NightOwl,
    Github,
    Selenized,
    Flexoki,
}

/// Whether a variant targets a dark or a light terminal background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

/// One row of the variant table: name, family, mode, and the variant label
/// shown next to the family in the settings menu.
type VariantRow = (ThemeName, ThemeFamily, ThemeMode, &'static str);

/// Single source of truth mapping every name onto its family and mode.
const VARIANTS: [VariantRow; 32] = [
    (ThemeName::Neon, ThemeFamily::Neon, ThemeMode::Dark, "Dark"),
    (
        ThemeName::NeonLight,
        ThemeFamily::Neon,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::CatppuccinMocha,
        ThemeFamily::Catppuccin,
        ThemeMode::Dark,
        "Mocha",
    ),
    (
        ThemeName::CatppuccinLatte,
        ThemeFamily::Catppuccin,
        ThemeMode::Light,
        "Latte",
    ),
    (
        ThemeName::SolarizedDark,
        ThemeFamily::Solarized,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::SolarizedLight,
        ThemeFamily::Solarized,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::TokyoNight,
        ThemeFamily::TokyoNight,
        ThemeMode::Dark,
        "Night",
    ),
    (
        ThemeName::TokyoNightDay,
        ThemeFamily::TokyoNight,
        ThemeMode::Light,
        "Day",
    ),
    (
        ThemeName::GruvboxDark,
        ThemeFamily::Gruvbox,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::GruvboxLight,
        ThemeFamily::Gruvbox,
        ThemeMode::Light,
        "Light",
    ),
    (ThemeName::Nord, ThemeFamily::Nord, ThemeMode::Dark, "Dark"),
    (
        ThemeName::NordLight,
        ThemeFamily::Nord,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::Dracula,
        ThemeFamily::Dracula,
        ThemeMode::Dark,
        "Dracula",
    ),
    (
        ThemeName::Alucard,
        ThemeFamily::Dracula,
        ThemeMode::Light,
        "Alucard",
    ),
    (
        ThemeName::OneDark,
        ThemeFamily::One,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::OneLight,
        ThemeFamily::One,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::RosePine,
        ThemeFamily::RosePine,
        ThemeMode::Dark,
        "Main",
    ),
    (
        ThemeName::RosePineDawn,
        ThemeFamily::RosePine,
        ThemeMode::Light,
        "Dawn",
    ),
    (
        ThemeName::KanagawaWave,
        ThemeFamily::Kanagawa,
        ThemeMode::Dark,
        "Wave",
    ),
    (
        ThemeName::KanagawaLotus,
        ThemeFamily::Kanagawa,
        ThemeMode::Light,
        "Lotus",
    ),
    (
        ThemeName::EverforestDark,
        ThemeFamily::Everforest,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::EverforestLight,
        ThemeFamily::Everforest,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::AyuDark,
        ThemeFamily::Ayu,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::AyuLight,
        ThemeFamily::Ayu,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::NightOwl,
        ThemeFamily::NightOwl,
        ThemeMode::Dark,
        "Night",
    ),
    (
        ThemeName::LightOwl,
        ThemeFamily::NightOwl,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::GithubDark,
        ThemeFamily::Github,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::GithubLight,
        ThemeFamily::Github,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::SelenizedDark,
        ThemeFamily::Selenized,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::SelenizedLight,
        ThemeFamily::Selenized,
        ThemeMode::Light,
        "Light",
    ),
    (
        ThemeName::FlexokiDark,
        ThemeFamily::Flexoki,
        ThemeMode::Dark,
        "Dark",
    ),
    (
        ThemeName::FlexokiLight,
        ThemeFamily::Flexoki,
        ThemeMode::Light,
        "Light",
    ),
];

/// The variant-table row for `name`; total and exhaustive by construction.
fn row(name: ThemeName) -> &'static VariantRow {
    VARIANTS
        .iter()
        .find(|(candidate, ..)| *candidate == name)
        .expect("every ThemeName appears in VARIANTS")
}

impl ThemeName {
    /// Every selectable variant, grouped by family in settings-menu order.
    pub const ALL: [ThemeName; 32] = {
        let mut all = [ThemeName::Neon; 32];
        let mut index = 0;
        while index < VARIANTS.len() {
            all[index] = VARIANTS[index].0;
            index += 1;
        }
        all
    };

    /// The family this variant belongs to.
    pub fn family(self) -> ThemeFamily {
        row(self).1
    }

    /// Whether this variant is dark or light.
    pub fn mode(self) -> ThemeMode {
        row(self).2
    }

    /// The variant label shown next to the family name ("Mocha", "Dawn").
    pub fn variant_label(self) -> &'static str {
        row(self).3
    }
}

impl ThemeFamily {
    /// Every family, in settings-menu order.
    pub const ALL: [ThemeFamily; 16] = [
        ThemeFamily::Neon,
        ThemeFamily::Catppuccin,
        ThemeFamily::Solarized,
        ThemeFamily::TokyoNight,
        ThemeFamily::Gruvbox,
        ThemeFamily::Nord,
        ThemeFamily::Dracula,
        ThemeFamily::One,
        ThemeFamily::RosePine,
        ThemeFamily::Kanagawa,
        ThemeFamily::Everforest,
        ThemeFamily::Ayu,
        ThemeFamily::NightOwl,
        ThemeFamily::Github,
        ThemeFamily::Selenized,
        ThemeFamily::Flexoki,
    ];

    /// Human-readable family name shown in the settings menu.
    pub fn label(self) -> &'static str {
        match self {
            ThemeFamily::Neon => "Neon",
            ThemeFamily::Catppuccin => "Catppuccin",
            ThemeFamily::Solarized => "Solarized",
            ThemeFamily::TokyoNight => "Tokyo Night",
            ThemeFamily::Gruvbox => "Gruvbox",
            ThemeFamily::Nord => "Nord",
            ThemeFamily::Dracula => "Dracula",
            ThemeFamily::One => "One",
            ThemeFamily::RosePine => "Rosé Pine",
            ThemeFamily::Kanagawa => "Kanagawa",
            ThemeFamily::Everforest => "Everforest",
            ThemeFamily::Ayu => "Ayu",
            ThemeFamily::NightOwl => "Night Owl",
            ThemeFamily::Github => "GitHub",
            ThemeFamily::Selenized => "Selenized",
            ThemeFamily::Flexoki => "Flexoki",
        }
    }

    /// This family's variant in `mode`.
    pub fn variant(self, mode: ThemeMode) -> ThemeName {
        VARIANTS
            .iter()
            .find(|(_, family, candidate_mode, _)| *family == self && *candidate_mode == mode)
            .expect("every family has a variant per mode")
            .0
    }
}

impl ThemeMode {
    /// The opposite mode.
    pub fn toggled(self) -> ThemeMode {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }

    /// Label shown in the settings menu mode switch.
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_round_trips_through_family_and_mode() {
        for name in ThemeName::ALL {
            assert_eq!(name.family().variant(name.mode()), name, "{name:?}");
        }
    }

    #[test]
    fn every_family_has_one_dark_and_one_light_variant() {
        for family in ThemeFamily::ALL {
            let dark = family.variant(ThemeMode::Dark);
            let light = family.variant(ThemeMode::Light);
            assert_ne!(dark, light, "{family:?}");
            assert_eq!(dark.mode(), ThemeMode::Dark, "{family:?}");
            assert_eq!(light.mode(), ThemeMode::Light, "{family:?}");
        }
    }

    #[test]
    fn the_variant_table_covers_every_name_exactly_once() {
        for name in ThemeName::ALL {
            assert_eq!(
                VARIANTS
                    .iter()
                    .filter(|(candidate, ..)| *candidate == name)
                    .count(),
                1,
                "{name:?}"
            );
        }
    }

    #[test]
    fn legacy_theme_names_keep_their_serde_spelling() {
        for (name, spelling) in [
            (ThemeName::Neon, "\"neon\""),
            (ThemeName::CatppuccinMocha, "\"catppuccin-mocha\""),
            (ThemeName::SolarizedDark, "\"solarized-dark\""),
            (ThemeName::TokyoNight, "\"tokyo-night\""),
            (ThemeName::GruvboxDark, "\"gruvbox-dark\""),
            (ThemeName::Nord, "\"nord\""),
            (ThemeName::Dracula, "\"dracula\""),
        ] {
            assert_eq!(serde_json::to_string(&name).expect("serialize"), spelling);
        }
    }
}
