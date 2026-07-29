use unicode_width::UnicodeWidthStr;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::{clip_end, transition_line};
use crate::app::state::AppState;
use crate::config::IconMode;
use crate::media::Track;
use crate::queue::RepeatMode;
use crate::ui::icons::icons_for;
use crate::ui::theme::Theme;

fn content(line: &ratatui::text::Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn playback_mode_colors(
    shuffle: bool,
    repeat: RepeatMode,
) -> (ratatui::style::Color, ratatui::style::Color) {
    let backend = TestBackend::new(100, 3);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let icons = icons_for(IconMode::Ascii);
    let mut state = AppState::new();
    state.domain.current_track = Some(Track::new("id", "Title", "Artist"));
    state.domain.queue.shuffle = shuffle;
    state.domain.queue.repeat = repeat;
    terminal
        .draw(|frame| {
            super::playback_summary(
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
fn transition_reveals_next_title_once_from_right() {
    let theme = Theme::from_truecolor(false);
    let icons = icons_for(IconMode::Ascii);
    let start = transition_line("Current", "Following", icons.chevron_l, 30, 0.0, &theme);
    let middle = transition_line("Current", "Following", icons.chevron_l, 30, 0.5, &theme);
    let finish = transition_line("Current", "Following", icons.chevron_l, 30, 1.0, &theme);

    assert_eq!(content(&start), "Current");
    let middle = content(&middle);
    assert!(middle.contains(" < Follo"));
    assert!(
        middle.find(" < ").expect("separator") > "Current".len(),
        "intermediate frame must travel in from the right: {middle:?}"
    );
    assert_eq!(content(&finish), "Current < Following");
}

#[test]
fn transition_uses_semantic_chevron_for_each_icon_mode() {
    let theme = Theme::from_truecolor(false);
    let ascii = transition_line(
        "A",
        "B",
        icons_for(IconMode::Ascii).chevron_l,
        12,
        1.0,
        &theme,
    );
    let nerd = transition_line(
        "A",
        "B",
        icons_for(IconMode::NerdFont).chevron_l,
        12,
        1.0,
        &theme,
    );
    assert_eq!(content(&ascii), "A < B");
    assert_eq!(content(&nerd), "A ‹ B");
}

#[test]
fn transition_preserves_current_and_next_styles() {
    let theme = Theme::from_truecolor(false);
    let line = transition_line("Current", "Next", "<", 20, 1.0, &theme);
    assert_eq!(line.spans[0].style.fg, theme.accent.fg);
    assert_eq!(
        line.spans.last().expect("next title").style.fg,
        theme.value.fg
    );
}

#[test]
fn transition_clips_by_terminal_cells_without_overflow() {
    let theme = Theme::from_truecolor(false);
    for width in 0..=80 {
        let line = transition_line(
            "Current 世界 with a long suffix",
            "Next 🎵 with a long suffix",
            "<",
            width,
            1.0,
            &theme,
        );
        assert!(content(&line).width() <= width, "width {width}: {line:?}");
    }
    assert_eq!(clip_end("世界", 3).width(), 3);
}
