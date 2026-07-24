use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::widgets::Paragraph;

use super::*;
use crate::config::IconMode;
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
                    visualizer::meter_line(&[0.5; BAND_COUNT], &theme, &icons),
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
fn ascii_components_do_not_leak_non_ascii_glyphs() {
    let out = render_component(100, icons_for(IconMode::Ascii));
    assert!(out.is_ascii(), "ASCII render leaked: {out:?}");
}
