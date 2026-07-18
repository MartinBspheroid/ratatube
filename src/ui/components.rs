//! Shared cyberpunk UI primitives used by responsive views.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use unicode_width::UnicodeWidthStr;

use crate::app::state::AppState;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;

/// Draw a dedicated title line and return the content rectangle below it.
pub fn section_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
    theme: &Theme,
    icons: &Icons,
) -> Rect {
    if area.is_empty() {
        return area;
    }
    frame.buffer_mut().set_style(area, theme.panel_bg);
    let marker = if focused {
        icons.play_btn
    } else {
        icons.section_bar
    };
    let title = title.to_ascii_uppercase();
    let occupied = marker.width() + 1 + title.width() + 1;
    let rule = icons
        .panel_rule
        .repeat((area.width as usize).saturating_sub(occupied));
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(marker, theme.panel_title),
            Span::raw(" "),
            Span::styled(
                title,
                if focused {
                    theme.panel_title.add_modifier(Modifier::REVERSED)
                } else {
                    theme.panel_title
                },
            ),
            Span::raw(" "),
            Span::styled(rule, theme.panel_rule),
        ])),
        Rect { height: 1, ..area },
    );
    Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    }
}

/// Build a link label and optional, truthful accelerator chip.
pub fn header_link<'a>(label: &'a str, key: Option<char>, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = vec![Span::styled(
        label,
        if key.is_some() { theme.link } else { theme.dim },
    )];
    if let Some(key) = key {
        spans.push(Span::styled(format!(" · {key}"), theme.key_chip));
    }
    spans
}

/// Render compact bracketed metadata chips.
pub fn chips(items: &[String], theme: &Theme) -> Line<'static> {
    Line::from(
        items
            .iter()
            .flat_map(|item| {
                [
                    Span::styled(format!("[{item}]"), theme.chip),
                    Span::raw(" "),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

/// Build aligned label/value rows.
pub fn key_value_rows(
    pairs: &[(String, String)],
    right_align: bool,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let label_width = pairs
        .iter()
        .map(|(label, _)| label.width())
        .max()
        .unwrap_or(0);
    pairs
        .iter()
        .map(|(label, value)| {
            let value = if right_align {
                format!("{value:>16}")
            } else {
                value.clone()
            };
            Line::from(vec![
                Span::styled(format!("{label:<label_width$}"), theme.label),
                Span::raw("  "),
                Span::styled(value, theme.value),
            ])
        })
        .collect()
}

/// Content and semantic state for a numbered list row.
pub struct NumberedRow<'a> {
    pub index: usize,
    pub title: &'a str,
    pub right_columns: &'a [String],
    pub playing: bool,
    pub selected: bool,
}

/// Build a width-safe numbered row with right-aligned secondary columns.
pub fn numbered_row(
    row: NumberedRow<'_>,
    width: usize,
    theme: &Theme,
    icons: &Icons,
) -> Line<'static> {
    let prefix = format!(
        "{} {:02} ",
        if row.playing { icons.play_btn } else { " " },
        row.index + 1
    );
    let right = row.right_columns.join("  ");
    let budget = width
        .saturating_sub(prefix.width())
        .saturating_sub(right.width())
        .saturating_sub(2);
    let title = crate::ui::widgets::truncate_end(row.title, budget);
    let padding = width
        .saturating_sub(prefix.width() + title.width() + right.width())
        .max(1);
    Line::from(vec![
        Span::styled(
            prefix,
            if row.playing {
                theme.success
            } else {
                theme.dim
            },
        ),
        Span::styled(
            title,
            if row.selected {
                theme.selected
            } else {
                theme.base
            },
        ),
        Span::raw(" ".repeat(padding)),
        Span::styled(right, theme.dim),
    ])
}

/// Build a compact one-row button with its actual accelerator.
pub fn button<'a>(label: &'a str, key: char, style: Style, theme: &Theme) -> Vec<Span<'a>> {
    vec![
        Span::styled("[", theme.dim),
        Span::styled(format!("{key}"), theme.key_chip),
        Span::raw(" "),
        Span::styled(label, style),
        Span::styled("]", theme.dim),
    ]
}

/// Build a deterministic animated spectrum from semantic icon ramp slots.
pub fn spectrum(
    width: usize,
    animation_frame: usize,
    theme: &Theme,
    icons: &Icons,
) -> Line<'static> {
    let spans = (0..width)
        .map(|column| {
            let index = (column.wrapping_mul(7) + animation_frame) % icons.spectrum_ramp.len();
            let style = if theme.truecolor {
                let ratio = column as f32 / width.max(1) as f32;
                let red = (34.0 + (217.0 - 34.0) * ratio) as u8;
                let green = (211.0 + (70.0 - 211.0) * ratio) as u8;
                let blue = (238.0 + (239.0 - 238.0) * ratio) as u8;
                Style::default().fg(Color::Rgb(red, green, blue))
            } else {
                theme.accent
            };
            Span::styled(icons.spectrum_ramp[index].to_string(), style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

/// Draw a standardized scrollbar only when content overflows its viewport.
pub fn scrollbar(frame: &mut Frame, area: Rect, content_len: usize, position: usize) {
    if content_len <= area.height as usize || area.is_empty() {
        return;
    }
    let mut state = ScrollbarState::new(content_len).position(position);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        area.inner(Margin {
            vertical: 0,
            horizontal: 0,
        }),
        &mut state,
    );
}

/// Render the shared three-row playback summary used by the mini player and
/// the Playing hero: name, full-width timeline, then time and playback modes.
pub fn playback_summary(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    icons: &Icons,
    theme: &Theme,
) {
    let Some(track) = &state.current_track else {
        frame.render_widget(
            Paragraph::new(Span::styled("Nothing is playing", theme.dim)),
            area,
        );
        return;
    };
    if area.is_empty() {
        return;
    }

    let channel = crate::ui::icons::sanitize_terminal_text(&track.artist);
    let title = crate::ui::icons::sanitize_terminal_text(&track.title);
    let name =
        crate::ui::widgets::truncate_middle(&format!("{title} — {channel}"), area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(name, theme.accent)),
        Rect { height: 1, ..area },
    );
    if area.height < 2 {
        return;
    }

    let position = state.playback.position_seconds;
    let duration = state.playback.duration_seconds.unwrap_or(0.0);
    let ratio = if duration > 0.0 {
        (position / duration).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let timeline_width = area.width as usize;
    let filled_width = (ratio * timeline_width as f64).round() as usize;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                symbols::line::THICK.horizontal.repeat(filled_width),
                theme.gauge_filled,
            ),
            Span::styled(
                symbols::line::THICK
                    .horizontal
                    .repeat(timeline_width.saturating_sub(filled_width)),
                theme.border,
            ),
        ])),
        Rect {
            y: area.y + 1,
            height: 1,
            ..area
        },
    );
    if area.height < 3 {
        return;
    }

    let status_area = Rect {
        y: area.y + 2,
        height: 1,
        ..area
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(status_area);
    let remaining = state
        .playback
        .duration_seconds
        .map(|total| (total - position).max(0.0));
    let time = format!(
        "{} / {}",
        crate::ui::widgets::format_time(position),
        remaining
            .map(crate::ui::widgets::format_time)
            .unwrap_or_else(|| "--:--".to_string())
    );
    frame.render_widget(Paragraph::new(Span::styled(time, theme.dim)), columns[0]);

    let shuffle_style = if state.queue.shuffle {
        theme.accent
    } else {
        theme.dim
    };
    let repeat_style = if state.queue.repeat == crate::queue::RepeatMode::Off {
        theme.dim
    } else {
        theme.accent
    };
    let volume_width = 8usize;
    let filled = usize::from(state.playback.volume.min(100)) * volume_width / 100;
    let volume_gauge = format!(
        "{}{}",
        icons.spectrum_ramp[3].repeat(filled),
        icons.spectrum_ramp[0].repeat(volume_width - filled)
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{}  ", icons.shuffle), shuffle_style),
            Span::styled(format!("{}  ", icons.repeat), repeat_style),
            Span::styled(format!("{} ", icons.volume), theme.dim),
            Span::styled(volume_gauge, theme.gauge_filled),
            Span::styled(format!(" {}%", state.playback.volume), theme.dim),
        ]))
        .alignment(ratatui::layout::Alignment::Right),
        columns[1],
    );
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;

    use super::*;
    use crate::config::IconMode;
    use crate::media::Track;
    use crate::queue::RepeatMode;
    use crate::ui::icons::icons_for;

    fn render_component(width: u16, icons: Icons) -> String {
        let backend = TestBackend::new(width, 6);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let theme = Theme::from_truecolor(false);
                let inner = section_panel(frame, frame.area(), "Now Playing", true, &theme, &icons);
                frame.render_widget(
                    Paragraph::new(vec![
                        numbered_row(
                            NumberedRow {
                                index: 0,
                                title: "A deliberately long title that must truncate",
                                right_columns: &["03:42".to_string()],
                                playing: true,
                                selected: true,
                            },
                            inner.width as usize,
                            &theme,
                            &icons,
                        ),
                        spectrum(inner.width.min(20) as usize, 3, &theme, &icons),
                    ]),
                    inner,
                );
            })
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn playback_mode_colors(shuffle: bool, repeat: RepeatMode) -> (Color, Color) {
        let backend = TestBackend::new(100, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let icons = icons_for(IconMode::Ascii);
        let mut state = AppState::new();
        state.current_track = Some(Track::new("id", "Title", "Artist"));
        state.queue.shuffle = shuffle;
        state.queue.repeat = repeat;
        terminal
            .draw(|frame| {
                playback_summary(
                    frame,
                    frame.area(),
                    &state,
                    &icons,
                    &Theme::from_truecolor(false),
                );
            })
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let row = (0..100)
            .map(|x| buffer.cell((x, 2)).expect("status cell").symbol())
            .collect::<String>();
        let shuffle_x = row.find("[SHUFFLE]").expect("shuffle label") as u16;
        let repeat_x = row.find("[REPEAT]").expect("repeat label") as u16;
        (
            buffer.cell((shuffle_x, 2)).expect("shuffle cell").fg,
            buffer.cell((repeat_x, 2)).expect("repeat cell").fg,
        )
    }

    #[test]
    fn components_render_at_floor_and_standard_widths() {
        for width in [40, 100] {
            let out = render_component(width, icons_for(IconMode::Ascii));
            assert!(out.contains("NOW PLAYING"));
            assert!(out.contains("03:42"));
        }
    }

    #[test]
    fn component_render_is_deterministic_for_same_frame() {
        let icons = icons_for(IconMode::Ascii);
        assert_eq!(render_component(100, icons), render_component(100, icons));
    }

    #[test]
    fn playback_modes_are_dim_when_off_and_cyan_when_on() {
        let theme = Theme::from_truecolor(false);
        let dim = theme.dim.fg.expect("dim foreground");
        let accent = theme.accent.fg.expect("accent foreground");
        assert_eq!(playback_mode_colors(false, RepeatMode::Off), (dim, dim));
        assert_eq!(
            playback_mode_colors(true, RepeatMode::Queue),
            (accent, accent)
        );
    }

    #[test]
    fn ascii_components_do_not_leak_non_ascii_glyphs() {
        let out = render_component(100, icons_for(IconMode::Ascii));
        assert!(out.is_ascii(), "ASCII render leaked: {out:?}");
    }
}
