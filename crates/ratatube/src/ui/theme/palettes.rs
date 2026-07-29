//! Truecolor swatches for every theme variant except dark Neon (which keeps
//! the legacy builder in the parent module). All values are the scheme's
//! official published colors; the doc comment on each constant names the
//! source roles where the mapping is not obvious.

use ratatui::style::Color;

use crate::config::ThemeName;

/// Truecolor swatch of one published color scheme, mapped onto the style
/// roles the UI consumes.
pub(super) struct Palette {
    pub bg: Color,
    pub panel_bg: Color,
    /// Emphasized foreground: headers, values, selected rows.
    pub text: Color,
    /// Secondary foreground: chips and de-emphasized copy.
    pub subtext: Color,
    /// Lowest-emphasis foreground: labels, rules, hints.
    pub dim: Color,
    pub accent: Color,
    pub accent_alt: Color,
    pub red: Color,
    pub yellow: Color,
    pub orange: Color,
    pub green: Color,
    pub selected_bg: Color,
    pub border: Color,
}

/// The palette for `name`; `None` only for dark Neon, whose palette is the
/// legacy `from_truecolor` builder.
pub(super) fn palette_for(name: ThemeName) -> Option<&'static Palette> {
    Some(match name {
        ThemeName::Neon => return None,
        ThemeName::NeonLight => &NEON_LIGHT,
        ThemeName::CatppuccinMocha => &CATPPUCCIN_MOCHA,
        ThemeName::CatppuccinLatte => &CATPPUCCIN_LATTE,
        ThemeName::SolarizedDark => &SOLARIZED_DARK,
        ThemeName::SolarizedLight => &SOLARIZED_LIGHT,
        ThemeName::TokyoNight => &TOKYO_NIGHT,
        ThemeName::TokyoNightDay => &TOKYO_NIGHT_DAY,
        ThemeName::GruvboxDark => &GRUVBOX_DARK,
        ThemeName::GruvboxLight => &GRUVBOX_LIGHT,
        ThemeName::Nord => &NORD,
        ThemeName::NordLight => &NORD_LIGHT,
        ThemeName::Dracula => &DRACULA,
        ThemeName::Alucard => &ALUCARD,
        ThemeName::OneDark => &ONE_DARK,
        ThemeName::OneLight => &ONE_LIGHT,
        ThemeName::RosePine => &ROSE_PINE,
        ThemeName::RosePineDawn => &ROSE_PINE_DAWN,
        ThemeName::KanagawaWave => &KANAGAWA_WAVE,
        ThemeName::KanagawaLotus => &KANAGAWA_LOTUS,
        ThemeName::EverforestDark => &EVERFOREST_DARK,
        ThemeName::EverforestLight => &EVERFOREST_LIGHT,
        ThemeName::AyuDark => &AYU_DARK,
        ThemeName::AyuLight => &AYU_LIGHT,
        ThemeName::NightOwl => &NIGHT_OWL,
        ThemeName::LightOwl => &LIGHT_OWL,
        ThemeName::GithubDark => &GITHUB_DARK,
        ThemeName::GithubLight => &GITHUB_LIGHT,
        ThemeName::SelenizedDark => &SELENIZED_DARK,
        ThemeName::SelenizedLight => &SELENIZED_LIGHT,
        ThemeName::FlexokiDark => &FLEXOKI_DARK,
        ThemeName::FlexokiLight => &FLEXOKI_LIGHT,
    })
}

/// Neon on paper: the only designed-in-house palette, keeping the family's
/// cyan/magenta identity darkened for light backgrounds.
const NEON_LIGHT: Palette = Palette {
    bg: Color::Rgb(238, 241, 246),
    panel_bg: Color::Rgb(248, 250, 253),
    text: Color::Rgb(15, 23, 42),
    subtext: Color::Rgb(71, 85, 105),
    dim: Color::Rgb(148, 163, 184),
    accent: Color::Rgb(14, 116, 144),
    accent_alt: Color::Rgb(162, 28, 175),
    red: Color::Rgb(220, 38, 38),
    yellow: Color::Rgb(217, 119, 6),
    orange: Color::Rgb(234, 88, 12),
    green: Color::Rgb(22, 163, 74),
    selected_bg: Color::Rgb(203, 231, 240),
    border: Color::Rgb(194, 203, 216),
};

/// Catppuccin Mocha: mantle canvas, base panels, mauve/pink accents.
const CATPPUCCIN_MOCHA: Palette = Palette {
    bg: Color::Rgb(24, 24, 37),
    panel_bg: Color::Rgb(30, 30, 46),
    text: Color::Rgb(205, 214, 244),
    subtext: Color::Rgb(166, 173, 200),
    dim: Color::Rgb(108, 112, 134),
    accent: Color::Rgb(203, 166, 247),
    accent_alt: Color::Rgb(245, 194, 231),
    red: Color::Rgb(243, 139, 168),
    yellow: Color::Rgb(249, 226, 175),
    orange: Color::Rgb(250, 179, 135),
    green: Color::Rgb(166, 227, 161),
    selected_bg: Color::Rgb(69, 71, 90),
    border: Color::Rgb(88, 91, 112),
};

/// Catppuccin Latte: the same roles from the light palette.
const CATPPUCCIN_LATTE: Palette = Palette {
    bg: Color::Rgb(230, 233, 239),
    panel_bg: Color::Rgb(239, 241, 245),
    text: Color::Rgb(76, 79, 105),
    subtext: Color::Rgb(108, 111, 133),
    dim: Color::Rgb(156, 160, 176),
    accent: Color::Rgb(136, 57, 239),
    accent_alt: Color::Rgb(234, 118, 203),
    red: Color::Rgb(210, 15, 57),
    yellow: Color::Rgb(223, 142, 29),
    orange: Color::Rgb(254, 100, 11),
    green: Color::Rgb(64, 160, 43),
    selected_bg: Color::Rgb(188, 192, 204),
    border: Color::Rgb(172, 176, 190),
};

/// Solarized Dark: base03 canvas, base02 panels, blue/cyan accents.
const SOLARIZED_DARK: Palette = Palette {
    bg: Color::Rgb(0, 43, 54),
    panel_bg: Color::Rgb(7, 54, 66),
    text: Color::Rgb(147, 161, 161),
    subtext: Color::Rgb(131, 148, 150),
    dim: Color::Rgb(88, 110, 117),
    accent: Color::Rgb(38, 139, 210),
    accent_alt: Color::Rgb(42, 161, 152),
    red: Color::Rgb(220, 50, 47),
    yellow: Color::Rgb(181, 137, 0),
    orange: Color::Rgb(203, 75, 22),
    green: Color::Rgb(133, 153, 0),
    selected_bg: Color::Rgb(88, 110, 117),
    border: Color::Rgb(88, 110, 117),
};

/// Solarized Light: the documented base inversion of Solarized Dark.
const SOLARIZED_LIGHT: Palette = Palette {
    bg: Color::Rgb(253, 246, 227),
    panel_bg: Color::Rgb(238, 232, 213),
    text: Color::Rgb(88, 110, 117),
    subtext: Color::Rgb(101, 123, 131),
    dim: Color::Rgb(147, 161, 161),
    accent: Color::Rgb(38, 139, 210),
    accent_alt: Color::Rgb(42, 161, 152),
    red: Color::Rgb(220, 50, 47),
    yellow: Color::Rgb(181, 137, 0),
    orange: Color::Rgb(203, 75, 22),
    green: Color::Rgb(133, 153, 0),
    selected_bg: Color::Rgb(147, 161, 161),
    border: Color::Rgb(147, 161, 161),
};

/// Tokyo Night: night canvas, storm panels, blue/purple accents.
const TOKYO_NIGHT: Palette = Palette {
    bg: Color::Rgb(26, 27, 38),
    panel_bg: Color::Rgb(36, 40, 59),
    text: Color::Rgb(192, 202, 245),
    subtext: Color::Rgb(169, 177, 214),
    dim: Color::Rgb(86, 95, 137),
    accent: Color::Rgb(122, 162, 247),
    accent_alt: Color::Rgb(187, 154, 247),
    red: Color::Rgb(247, 118, 142),
    yellow: Color::Rgb(224, 175, 104),
    orange: Color::Rgb(255, 158, 100),
    green: Color::Rgb(158, 206, 106),
    selected_bg: Color::Rgb(40, 52, 87),
    border: Color::Rgb(59, 66, 97),
};

/// Tokyo Night Day, from the tokyonight.nvim generated extras.
const TOKYO_NIGHT_DAY: Palette = Palette {
    bg: Color::Rgb(208, 213, 227),
    panel_bg: Color::Rgb(225, 226, 231),
    text: Color::Rgb(55, 96, 191),
    subtext: Color::Rgb(97, 114, 176),
    dim: Color::Rgb(132, 140, 181),
    accent: Color::Rgb(46, 125, 233),
    accent_alt: Color::Rgb(152, 84, 241),
    red: Color::Rgb(245, 42, 101),
    yellow: Color::Rgb(140, 108, 62),
    orange: Color::Rgb(177, 92, 0),
    green: Color::Rgb(88, 117, 57),
    selected_bg: Color::Rgb(183, 193, 227),
    border: Color::Rgb(180, 181, 185),
};

/// Gruvbox dark mode: warm grays with the signature bright orange.
const GRUVBOX_DARK: Palette = Palette {
    bg: Color::Rgb(40, 40, 40),
    panel_bg: Color::Rgb(60, 56, 54),
    text: Color::Rgb(235, 219, 178),
    subtext: Color::Rgb(168, 153, 132),
    dim: Color::Rgb(146, 131, 116),
    accent: Color::Rgb(254, 128, 25),
    accent_alt: Color::Rgb(142, 192, 124),
    red: Color::Rgb(251, 73, 52),
    yellow: Color::Rgb(250, 189, 47),
    orange: Color::Rgb(254, 128, 25),
    green: Color::Rgb(184, 187, 38),
    selected_bg: Color::Rgb(80, 73, 69),
    border: Color::Rgb(102, 92, 84),
};

/// Gruvbox light mode: light0/1 surfaces with the faded accent set.
const GRUVBOX_LIGHT: Palette = Palette {
    bg: Color::Rgb(251, 241, 199),
    panel_bg: Color::Rgb(235, 219, 178),
    text: Color::Rgb(60, 56, 54),
    subtext: Color::Rgb(102, 92, 84),
    dim: Color::Rgb(146, 131, 116),
    accent: Color::Rgb(175, 58, 3),
    accent_alt: Color::Rgb(66, 123, 88),
    red: Color::Rgb(157, 0, 6),
    yellow: Color::Rgb(181, 118, 20),
    orange: Color::Rgb(175, 58, 3),
    green: Color::Rgb(121, 116, 14),
    selected_bg: Color::Rgb(213, 196, 161),
    border: Color::Rgb(189, 174, 147),
};

/// Nord: polar-night surfaces, frost accents, aurora status colors.
const NORD: Palette = Palette {
    bg: Color::Rgb(46, 52, 64),
    panel_bg: Color::Rgb(59, 66, 82),
    text: Color::Rgb(236, 239, 244),
    subtext: Color::Rgb(216, 222, 233),
    dim: Color::Rgb(76, 86, 106),
    accent: Color::Rgb(136, 192, 208),
    accent_alt: Color::Rgb(180, 142, 173),
    red: Color::Rgb(191, 97, 106),
    yellow: Color::Rgb(235, 203, 139),
    orange: Color::Rgb(208, 135, 112),
    green: Color::Rgb(163, 190, 140),
    selected_bg: Color::Rgb(67, 76, 94),
    border: Color::Rgb(76, 86, 106),
};

/// Nord on Snow Storm, per the documented bright-design role swap; nord10
/// replaces nord8 as the accent for contrast, and warning uses nord12
/// because nord13 is unreadable on white.
const NORD_LIGHT: Palette = Palette {
    bg: Color::Rgb(236, 239, 244),
    panel_bg: Color::Rgb(229, 233, 240),
    text: Color::Rgb(46, 52, 64),
    subtext: Color::Rgb(59, 66, 82),
    dim: Color::Rgb(76, 86, 106),
    accent: Color::Rgb(94, 129, 172),
    accent_alt: Color::Rgb(180, 142, 173),
    red: Color::Rgb(191, 97, 106),
    yellow: Color::Rgb(208, 135, 112),
    orange: Color::Rgb(208, 135, 112),
    green: Color::Rgb(163, 190, 140),
    selected_bg: Color::Rgb(216, 222, 233),
    border: Color::Rgb(216, 222, 233),
};

/// Dracula: the official spec palette with purple/pink accents.
const DRACULA: Palette = Palette {
    bg: Color::Rgb(33, 34, 44),
    panel_bg: Color::Rgb(40, 42, 54),
    text: Color::Rgb(248, 248, 242),
    subtext: Color::Rgb(98, 114, 164),
    dim: Color::Rgb(98, 114, 164),
    accent: Color::Rgb(189, 147, 249),
    accent_alt: Color::Rgb(255, 121, 198),
    red: Color::Rgb(255, 85, 85),
    yellow: Color::Rgb(241, 250, 140),
    orange: Color::Rgb(255, 184, 108),
    green: Color::Rgb(80, 250, 123),
    selected_bg: Color::Rgb(68, 71, 90),
    border: Color::Rgb(98, 114, 164),
};

/// Alucard, Dracula's official light theme; the panel is the background
/// cream darkened one step because the spec defines no elevated surface.
const ALUCARD: Palette = Palette {
    bg: Color::Rgb(255, 251, 235),
    panel_bg: Color::Rgb(245, 240, 222),
    text: Color::Rgb(31, 31, 31),
    subtext: Color::Rgb(108, 102, 75),
    dim: Color::Rgb(108, 102, 75),
    accent: Color::Rgb(100, 74, 201),
    accent_alt: Color::Rgb(163, 20, 77),
    red: Color::Rgb(203, 58, 42),
    yellow: Color::Rgb(132, 110, 21),
    orange: Color::Rgb(163, 77, 20),
    green: Color::Rgb(20, 113, 10),
    selected_bg: Color::Rgb(207, 207, 222),
    border: Color::Rgb(207, 207, 222),
};

/// Atom One Dark: the mono-1/2/3 foreground scale with the hue set.
const ONE_DARK: Palette = Palette {
    bg: Color::Rgb(33, 37, 43),
    panel_bg: Color::Rgb(40, 44, 52),
    text: Color::Rgb(171, 178, 191),
    subtext: Color::Rgb(130, 137, 151),
    dim: Color::Rgb(92, 99, 112),
    accent: Color::Rgb(97, 175, 239),
    accent_alt: Color::Rgb(198, 120, 221),
    red: Color::Rgb(224, 108, 117),
    yellow: Color::Rgb(229, 192, 123),
    orange: Color::Rgb(209, 154, 102),
    green: Color::Rgb(152, 195, 121),
    selected_bg: Color::Rgb(62, 68, 81),
    border: Color::Rgb(75, 82, 99),
};

/// Atom One Light: the mirrored mono scale and hue set.
const ONE_LIGHT: Palette = Palette {
    bg: Color::Rgb(234, 234, 235),
    panel_bg: Color::Rgb(250, 250, 250),
    text: Color::Rgb(56, 58, 66),
    subtext: Color::Rgb(105, 108, 119),
    dim: Color::Rgb(160, 161, 167),
    accent: Color::Rgb(64, 120, 242),
    accent_alt: Color::Rgb(166, 38, 164),
    red: Color::Rgb(228, 86, 73),
    yellow: Color::Rgb(193, 132, 1),
    orange: Color::Rgb(152, 104, 1),
    green: Color::Rgb(80, 161, 79),
    selected_bg: Color::Rgb(229, 229, 230),
    border: Color::Rgb(157, 157, 159),
};

/// Rosé Pine main: iris/foam accents; pine stands in for green because the
/// palette defines no true green.
const ROSE_PINE: Palette = Palette {
    bg: Color::Rgb(25, 23, 36),
    panel_bg: Color::Rgb(31, 29, 46),
    text: Color::Rgb(224, 222, 244),
    subtext: Color::Rgb(144, 140, 170),
    dim: Color::Rgb(110, 106, 134),
    accent: Color::Rgb(196, 167, 231),
    accent_alt: Color::Rgb(156, 207, 216),
    red: Color::Rgb(235, 111, 146),
    yellow: Color::Rgb(246, 193, 119),
    orange: Color::Rgb(235, 188, 186),
    green: Color::Rgb(49, 116, 143),
    selected_bg: Color::Rgb(64, 61, 82),
    border: Color::Rgb(82, 79, 103),
};

/// Rosé Pine Dawn: the same roles from the dawn palette.
const ROSE_PINE_DAWN: Palette = Palette {
    bg: Color::Rgb(250, 244, 237),
    panel_bg: Color::Rgb(255, 250, 243),
    text: Color::Rgb(87, 82, 121),
    subtext: Color::Rgb(121, 117, 147),
    dim: Color::Rgb(152, 147, 165),
    accent: Color::Rgb(144, 122, 169),
    accent_alt: Color::Rgb(86, 148, 159),
    red: Color::Rgb(180, 99, 122),
    yellow: Color::Rgb(234, 157, 52),
    orange: Color::Rgb(215, 130, 126),
    green: Color::Rgb(40, 105, 131),
    selected_bg: Color::Rgb(223, 218, 217),
    border: Color::Rgb(206, 202, 205),
};

/// Kanagawa Wave: sumi ink surfaces with crystal blue and oni violet.
const KANAGAWA_WAVE: Palette = Palette {
    bg: Color::Rgb(31, 31, 40),
    panel_bg: Color::Rgb(42, 42, 55),
    text: Color::Rgb(220, 215, 186),
    subtext: Color::Rgb(200, 192, 147),
    dim: Color::Rgb(114, 113, 105),
    accent: Color::Rgb(126, 156, 216),
    accent_alt: Color::Rgb(149, 127, 184),
    red: Color::Rgb(255, 93, 98),
    yellow: Color::Rgb(230, 195, 132),
    orange: Color::Rgb(255, 160, 102),
    green: Color::Rgb(152, 187, 108),
    selected_bg: Color::Rgb(34, 50, 73),
    border: Color::Rgb(84, 84, 109),
};

/// Kanagawa Lotus: lotus white surfaces with the lotus accent set.
const KANAGAWA_LOTUS: Palette = Palette {
    bg: Color::Rgb(242, 236, 188),
    panel_bg: Color::Rgb(229, 221, 176),
    text: Color::Rgb(84, 84, 100),
    subtext: Color::Rgb(67, 67, 108),
    dim: Color::Rgb(138, 137, 128),
    accent: Color::Rgb(77, 105, 155),
    accent_alt: Color::Rgb(98, 76, 131),
    red: Color::Rgb(200, 64, 83),
    yellow: Color::Rgb(222, 152, 0),
    orange: Color::Rgb(204, 109, 0),
    green: Color::Rgb(111, 137, 78),
    selected_bg: Color::Rgb(228, 215, 148),
    border: Color::Rgb(160, 156, 172),
};

/// Everforest dark (medium): green is both the identity accent and the
/// playing color, as in the scheme itself.
const EVERFOREST_DARK: Palette = Palette {
    bg: Color::Rgb(35, 42, 46),
    panel_bg: Color::Rgb(45, 53, 59),
    text: Color::Rgb(211, 198, 170),
    subtext: Color::Rgb(157, 169, 160),
    dim: Color::Rgb(122, 132, 120),
    accent: Color::Rgb(167, 192, 128),
    accent_alt: Color::Rgb(131, 192, 146),
    red: Color::Rgb(230, 126, 128),
    yellow: Color::Rgb(219, 188, 127),
    orange: Color::Rgb(230, 152, 117),
    green: Color::Rgb(167, 192, 128),
    selected_bg: Color::Rgb(61, 72, 77),
    border: Color::Rgb(86, 99, 95),
};

/// Everforest light (medium): the mirrored role set.
const EVERFOREST_LIGHT: Palette = Palette {
    bg: Color::Rgb(239, 235, 212),
    panel_bg: Color::Rgb(253, 246, 227),
    text: Color::Rgb(92, 106, 114),
    subtext: Color::Rgb(130, 145, 129),
    dim: Color::Rgb(166, 176, 160),
    accent: Color::Rgb(141, 161, 1),
    accent_alt: Color::Rgb(53, 167, 124),
    red: Color::Rgb(248, 85, 82),
    yellow: Color::Rgb(223, 160, 0),
    orange: Color::Rgb(245, 125, 38),
    green: Color::Rgb(141, 161, 1),
    selected_bg: Color::Rgb(230, 226, 204),
    border: Color::Rgb(189, 195, 175),
};

/// Ayu Dark, from the official vim port's palette.
const AYU_DARK: Palette = Palette {
    bg: Color::Rgb(15, 20, 25),
    panel_bg: Color::Rgb(20, 25, 31),
    text: Color::Rgb(230, 225, 207),
    subtext: Color::Rgb(92, 103, 115),
    dim: Color::Rgb(92, 103, 115),
    accent: Color::Rgb(242, 151, 24),
    accent_alt: Color::Rgb(54, 163, 217),
    red: Color::Rgb(240, 113, 120),
    yellow: Color::Rgb(231, 197, 71),
    orange: Color::Rgb(255, 119, 51),
    green: Color::Rgb(184, 204, 82),
    selected_bg: Color::Rgb(37, 51, 64),
    border: Color::Rgb(45, 54, 64),
};

/// Ayu Light: the light accent (#ff6a00) with the light syntax set.
const AYU_LIGHT: Palette = Palette {
    bg: Color::Rgb(250, 250, 250),
    panel_bg: Color::Rgb(255, 255, 255),
    text: Color::Rgb(92, 103, 115),
    subtext: Color::Rgb(171, 176, 182),
    dim: Color::Rgb(171, 176, 182),
    accent: Color::Rgb(255, 106, 0),
    accent_alt: Color::Rgb(54, 163, 217),
    red: Color::Rgb(255, 51, 51),
    yellow: Color::Rgb(242, 151, 24),
    orange: Color::Rgb(255, 119, 51),
    green: Color::Rgb(134, 179, 0),
    selected_bg: Color::Rgb(240, 238, 228),
    border: Color::Rgb(217, 216, 215),
};

/// Night Owl: deep navy with the theme's blue/purple/peach accents.
const NIGHT_OWL: Palette = Palette {
    bg: Color::Rgb(1, 22, 39),
    panel_bg: Color::Rgb(11, 41, 66),
    text: Color::Rgb(214, 222, 235),
    subtext: Color::Rgb(139, 173, 193),
    dim: Color::Rgb(99, 119, 119),
    accent: Color::Rgb(130, 170, 255),
    accent_alt: Color::Rgb(199, 146, 234),
    red: Color::Rgb(255, 88, 116),
    yellow: Color::Rgb(236, 196, 141),
    orange: Color::Rgb(247, 140, 108),
    green: Color::Rgb(197, 228, 120),
    selected_bg: Color::Rgb(29, 59, 83),
    border: Color::Rgb(95, 126, 151),
};

/// Light Owl: teal stands in for green, as the theme uses it for success
/// roles.
const LIGHT_OWL: Palette = Palette {
    bg: Color::Rgb(240, 240, 240),
    panel_bg: Color::Rgb(251, 251, 251),
    text: Color::Rgb(64, 63, 83),
    subtext: Color::Rgb(144, 167, 178),
    dim: Color::Rgb(152, 159, 177),
    accent: Color::Rgb(72, 118, 214),
    accent_alt: Color::Rgb(153, 76, 195),
    red: Color::Rgb(201, 103, 101),
    yellow: Color::Rgb(218, 170, 1),
    orange: Color::Rgb(188, 84, 84),
    green: Color::Rgb(12, 150, 155),
    selected_bg: Color::Rgb(224, 224, 224),
    border: Color::Rgb(217, 217, 217),
};

/// GitHub Dark, from the Primer color system.
const GITHUB_DARK: Palette = Palette {
    bg: Color::Rgb(13, 17, 23),
    panel_bg: Color::Rgb(22, 27, 34),
    text: Color::Rgb(230, 237, 243),
    subtext: Color::Rgb(139, 148, 158),
    dim: Color::Rgb(110, 118, 129),
    accent: Color::Rgb(88, 166, 255),
    accent_alt: Color::Rgb(188, 140, 255),
    red: Color::Rgb(248, 81, 73),
    yellow: Color::Rgb(210, 153, 34),
    orange: Color::Rgb(219, 109, 40),
    green: Color::Rgb(63, 185, 80),
    selected_bg: Color::Rgb(48, 54, 61),
    border: Color::Rgb(48, 54, 61),
};

/// GitHub Light, from the Primer color system.
const GITHUB_LIGHT: Palette = Palette {
    bg: Color::Rgb(246, 248, 250),
    panel_bg: Color::Rgb(255, 255, 255),
    text: Color::Rgb(36, 41, 47),
    subtext: Color::Rgb(87, 96, 106),
    dim: Color::Rgb(110, 119, 129),
    accent: Color::Rgb(9, 105, 218),
    accent_alt: Color::Rgb(130, 80, 223),
    red: Color::Rgb(207, 34, 46),
    yellow: Color::Rgb(154, 103, 0),
    orange: Color::Rgb(188, 76, 0),
    green: Color::Rgb(26, 127, 55),
    selected_bg: Color::Rgb(221, 244, 255),
    border: Color::Rgb(208, 215, 222),
};

/// Selenized dark: the bg_0/1/2 scale with fg_0/1 and the hue set.
const SELENIZED_DARK: Palette = Palette {
    bg: Color::Rgb(16, 60, 72),
    panel_bg: Color::Rgb(24, 73, 86),
    text: Color::Rgb(202, 216, 217),
    subtext: Color::Rgb(173, 188, 188),
    dim: Color::Rgb(114, 137, 143),
    accent: Color::Rgb(70, 149, 247),
    accent_alt: Color::Rgb(175, 136, 235),
    red: Color::Rgb(250, 87, 80),
    yellow: Color::Rgb(219, 179, 45),
    orange: Color::Rgb(237, 134, 73),
    green: Color::Rgb(117, 185, 56),
    selected_bg: Color::Rgb(45, 91, 105),
    border: Color::Rgb(45, 91, 105),
};

/// Selenized light: the mirrored scale.
const SELENIZED_LIGHT: Palette = Palette {
    bg: Color::Rgb(251, 243, 219),
    panel_bg: Color::Rgb(236, 227, 204),
    text: Color::Rgb(58, 77, 83),
    subtext: Color::Rgb(83, 103, 109),
    dim: Color::Rgb(144, 153, 149),
    accent: Color::Rgb(0, 114, 212),
    accent_alt: Color::Rgb(135, 98, 198),
    red: Color::Rgb(210, 33, 45),
    yellow: Color::Rgb(173, 137, 0),
    orange: Color::Rgb(194, 93, 30),
    green: Color::Rgb(72, 145, 0),
    selected_bg: Color::Rgb(213, 205, 182),
    border: Color::Rgb(213, 205, 182),
};

/// Flexoki dark: base-scale surfaces with the 400-tier accents.
const FLEXOKI_DARK: Palette = Palette {
    bg: Color::Rgb(16, 15, 15),
    panel_bg: Color::Rgb(28, 27, 26),
    text: Color::Rgb(206, 205, 195),
    subtext: Color::Rgb(135, 133, 128),
    dim: Color::Rgb(111, 110, 105),
    accent: Color::Rgb(67, 133, 190),
    accent_alt: Color::Rgb(206, 93, 151),
    red: Color::Rgb(209, 77, 65),
    yellow: Color::Rgb(208, 162, 21),
    orange: Color::Rgb(218, 112, 44),
    green: Color::Rgb(135, 154, 57),
    selected_bg: Color::Rgb(52, 51, 49),
    border: Color::Rgb(64, 62, 60),
};

/// Flexoki light: paper surfaces with the 600-tier accents.
const FLEXOKI_LIGHT: Palette = Palette {
    bg: Color::Rgb(255, 252, 240),
    panel_bg: Color::Rgb(242, 240, 229),
    text: Color::Rgb(16, 15, 15),
    subtext: Color::Rgb(87, 86, 83),
    dim: Color::Rgb(135, 133, 128),
    accent: Color::Rgb(32, 94, 166),
    accent_alt: Color::Rgb(160, 47, 111),
    red: Color::Rgb(175, 48, 41),
    yellow: Color::Rgb(173, 131, 1),
    orange: Color::Rgb(188, 82, 21),
    green: Color::Rgb(102, 128, 11),
    selected_bg: Color::Rgb(218, 216, 206),
    border: Color::Rgb(206, 205, 195),
};
