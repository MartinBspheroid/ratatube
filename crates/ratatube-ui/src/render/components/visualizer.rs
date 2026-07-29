//! Six-band level meter driven by real audio measurements.
//!
//! mpv exposes no FFT, so the bands are shaped from three genuine signals
//! per window: RMS energy (overall height), zero-crossings rate (spectral
//! brightness placing the hump between bass and treble bands), and crest
//! factor (peak minus RMS, spiking the hump on transients).

use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::render::icons::Icons;
use crate::render::theme::Theme;
use ratatube_domain::playback::AudioLevels;

/// Number of meter bands.
pub const BAND_COUNT: usize = 6;

/// Rendered width of the meter in cells (bands separated by one space).
pub const METER_WIDTH: u16 = (BAND_COUNT * 2 - 1) as u16;

/// Exponential release factor per update; attack is instantaneous.
const RELEASE: f32 = 0.72;

/// Quietest RMS level the meter still shows (dBFS).
const FLOOR_DB: f32 = -60.0;

/// Shape target band heights (0..1) from one measurement window.
pub fn bands_for(levels: AudioLevels) -> [f32; BAND_COUNT] {
    let energy = ((levels.rms_db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
    // Music ZCR rarely exceeds ~0.25; map it onto the band axis.
    let brightness = (levels.zcr * 4.0).clamp(0.0, 1.0);
    let crest = ((levels.peak_db - levels.rms_db) / 20.0).clamp(0.0, 1.0);
    let center = brightness * (BAND_COUNT - 1) as f32;
    let mut bands = [0.0; BAND_COUNT];
    for (index, band) in bands.iter_mut().enumerate() {
        let distance = index as f32 - center;
        let hump = (-(distance * distance) / 3.2).exp();
        *band = (energy * (0.25 + 0.75 * hump) * (0.8 + 0.4 * crest)).clamp(0.0, 1.0);
    }
    bands
}

/// Advance displayed heights toward `target`: instant attack, smooth release.
pub fn smooth(displayed: &mut [f32; BAND_COUNT], target: &[f32; BAND_COUNT]) {
    for (shown, wanted) in displayed.iter_mut().zip(target) {
        *shown = wanted.max(*shown * RELEASE);
        if *shown < 0.02 {
            *shown = 0.0;
        }
    }
}

/// Render the meter as one line of ramp glyphs colored along the theme's
/// accent gradient.
pub fn meter_line(bands: &[f32; BAND_COUNT], theme: &Theme, icons: &Icons) -> Line<'static> {
    let ramp = icons.spectrum_ramp;
    let mut spans = Vec::with_capacity(BAND_COUNT * 2 - 1);
    for (index, band) in bands.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let glyph = if *band <= 0.0 {
            " ".to_string()
        } else {
            let step = ((band * ramp.len() as f32).ceil() as usize).clamp(1, ramp.len());
            ramp[step - 1].to_string()
        };
        let ratio = index as f32 / (BAND_COUNT - 1) as f32;
        spans.push(Span::styled(glyph, band_style(ratio, theme)));
    }
    Line::from(spans)
}

/// Interpolate between the theme's two accents; non-RGB themes (the ANSI
/// fallback) keep the flat accent style.
fn band_style(ratio: f32, theme: &Theme) -> ratatui::style::Style {
    match (theme.accent.fg, theme.accent_alt.fg) {
        (Some(Color::Rgb(r1, g1, b1)), Some(Color::Rgb(r2, g2, b2))) => {
            let lerp = |a: u8, b: u8| -> u8 {
                (f32::from(a) + (f32::from(b) - f32::from(a)) * ratio).round() as u8
            };
            ratatui::style::Style::default().fg(Color::Rgb(
                lerp(r1, r2),
                lerp(g1, g2),
                lerp(b1, b2),
            ))
        }
        _ => theme.accent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(rms_db: f32, peak_db: f32, zcr: f32) -> AudioLevels {
        AudioLevels {
            rms_db,
            peak_db,
            zcr,
        }
    }

    #[test]
    fn silence_produces_no_bands() {
        let bands = bands_for(levels(-90.0, -90.0, 0.0));
        assert!(bands.iter().all(|band| *band == 0.0), "{bands:?}");
    }

    #[test]
    fn bass_heavy_audio_peaks_in_the_first_band() {
        let bands = bands_for(levels(-10.0, -5.0, 0.01));
        let max = bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .expect("bands");
        assert_eq!(max.0, 0, "{bands:?}");
        assert!(bands[0] > 0.5, "{bands:?}");
    }

    #[test]
    fn bright_audio_peaks_in_the_last_band() {
        let bands = bands_for(levels(-10.0, -5.0, 0.30));
        let max = bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .expect("bands");
        assert_eq!(max.0, BAND_COUNT - 1, "{bands:?}");
    }

    #[test]
    fn smoothing_attacks_instantly_and_releases_gradually() {
        let mut shown = [0.0; BAND_COUNT];
        smooth(&mut shown, &[1.0; BAND_COUNT]);
        assert_eq!(shown, [1.0; BAND_COUNT]);
        smooth(&mut shown, &[0.0; BAND_COUNT]);
        assert!(shown[0] > 0.5 && shown[0] < 1.0, "{shown:?}");
        for _ in 0..40 {
            smooth(&mut shown, &[0.0; BAND_COUNT]);
        }
        assert_eq!(shown, [0.0; BAND_COUNT]);
    }

    #[test]
    fn meter_line_spans_the_documented_width() {
        let theme = Theme::from_truecolor(true);
        let icons = crate::render::icons::icons_for(ratatube_domain::config::IconMode::Ascii);
        let line = meter_line(&[0.5; BAND_COUNT], &theme, &icons);
        let width: usize = line.spans.iter().map(|span| span.content.len()).sum();
        assert_eq!(width, METER_WIDTH as usize);
    }
}
