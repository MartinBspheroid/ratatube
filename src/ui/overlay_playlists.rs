//! Prompt, playlist-editor, picker, and confirmation overlays.

use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::app::state::{AppState, PlaylistEditorField, PromptPurpose};
use crate::ui::{centered_rect, icons, theme};

/// Render playlist-owned overlays, returning whether one was active.
pub(super) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    icon_set: &icons::Icons,
    theme: &theme::Theme,
) -> bool {
    if let Some(prompt) = &state.prompt {
        let title = match prompt.purpose {
            PromptPurpose::SaveQueueAsPlaylist => "Save queue as playlist",
            PromptPurpose::RenamePlaylist => "Rename playlist",
            PromptPurpose::ImportPlaylistUrl => "Import playlist URL",
            PromptPurpose::ImportPlaylistJson => "Paste playlist JSON",
            PromptPurpose::NewPlaylist => "New playlist name",
        };
        let json_prompt = prompt.purpose == PromptPurpose::ImportPlaylistJson;
        let prompt_area = if json_prompt {
            centered_rect(area, 76, 18)
        } else {
            centered_rect(area, 56, 5)
        };
        frame.render_widget(Clear, prompt_area);
        if json_prompt {
            let content = Paragraph::new(icons::sanitize_terminal_text(&prompt.buffer))
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(modal_block(title, theme).title_bottom(Line::from(vec![
                    Span::styled(" Enter ", theme.key_chip),
                    Span::styled("import  ", theme.dim),
                    Span::styled(" Esc ", theme.key_chip),
                    Span::styled("cancel", theme.dim),
                ])));
            frame.render_widget(content, prompt_area);
        } else {
            let line = Line::from(vec![
                Span::raw(icons::sanitize_terminal_text(&prompt.buffer)),
                Span::styled(icon_set.section_bar, theme.accent),
            ]);
            frame.render_widget(
                Paragraph::new(line).block(modal_block(title, theme)),
                prompt_area,
            );
        }
        return true;
    }
    if let Some(editor) = &state.playlist_editor {
        render_editor(frame, area, editor, theme);
        return true;
    }
    if let Some(picker) = &state.picker {
        render_picker(frame, area, state, picker, icon_set, theme);
        return true;
    }
    if let Some(confirm) = &state.confirm {
        let confirm_area = centered_rect(area, 50, 5);
        frame.render_widget(Clear, confirm_area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(icons::sanitize_terminal_text(&confirm.message)),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" y ", theme.key_chip),
                    Span::styled("yes   ", theme.dim),
                    Span::styled(" n ", theme.key_chip),
                    Span::styled("no", theme.dim),
                ]),
            ])
            .block(modal_block("Confirm", theme)),
            confirm_area,
        );
        return true;
    }
    false
}

fn render_editor(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    editor: &crate::app::state::PlaylistEditorState,
    theme: &theme::Theme,
) {
    let editor_area = centered_rect(area, 72, 14);
    let fields = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Length(1),
    ])
    .spacing(1)
    .split(editor_area.inner(ratatui::layout::Margin::new(2, 2)));
    frame.render_widget(Clear, editor_area);
    frame.render_widget(modal_block("Edit playlist", theme), editor_area);
    for (index, (title, value, active)) in [
        (
            "Name",
            editor.name.as_str(),
            editor.field == PlaylistEditorField::Name,
        ),
        (
            "Description",
            editor.description.as_str(),
            editor.field == PlaylistEditorField::Description,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let block = Block::bordered()
            .title(format!(" {title} "))
            .border_style(if active { theme.accent } else { theme.border })
            .style(if active { theme.selected } else { theme.base });
        frame.render_widget(
            Paragraph::new(icons::sanitize_terminal_text(value)).block(block),
            fields[index],
        );
    }
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Tab ", theme.key_chip),
            Span::styled("next field  ", theme.dim),
            Span::styled(" Enter ", theme.key_chip),
            Span::styled("save  ", theme.dim),
            Span::styled(" Esc ", theme.key_chip),
            Span::styled("cancel", theme.dim),
        ])),
        fields[2],
    );
}

fn render_picker(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &AppState,
    picker: &crate::app::state::PickerState,
    icon_set: &icons::Icons,
    theme: &theme::Theme,
) {
    let (create_new, matching) =
        crate::app::filter::picker_candidates(&state.playlists, &picker.filter);
    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled("> ", theme.accent),
        Span::raw(icons::sanitize_terminal_text(&picker.filter)),
        Span::styled(icon_set.section_bar, theme.accent),
    ])];
    let mut row = 0usize;
    let mut push_candidate = |label: Vec<Span<'static>>, selected: bool| {
        let mut spans = vec![Span::styled(
            if selected { icon_set.play_btn } else { "  " },
            theme.accent,
        )];
        spans.extend(label);
        let line = Line::from(spans);
        lines.push(if selected {
            line.style(theme.selected)
        } else {
            line
        });
    };
    if create_new {
        push_candidate(
            vec![Span::styled(
                format!("＋ New playlist \"{}\"", picker.filter.trim()),
                theme.playing,
            )],
            picker.selected == row,
        );
        row += 1;
    }
    for &index in matching.iter().take(9) {
        if let Some(playlist) = state.playlists.get(index) {
            push_candidate(
                vec![
                    Span::styled(icons::sanitize_terminal_text(&playlist.name), theme.base),
                    Span::styled(format!("  ({})", playlist.tracks.len()), theme.dim),
                ],
                picker.selected == row,
            );
            row += 1;
        }
    }
    if row == 0 {
        lines.push(Line::from(Span::styled(
            "No matching playlists — type a name to create one",
            theme.dim,
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Enter ", theme.key_chip),
        Span::styled("add   ", theme.dim),
        Span::styled(" ↑/↓ ", theme.key_chip),
        Span::styled("choose   ", theme.dim),
        Span::styled(" Esc ", theme.key_chip),
        Span::styled("cancel", theme.dim),
    ]));
    let picker_area = centered_rect(area, 56, (lines.len() as u16 + 2).min(area.height));
    frame.render_widget(Clear, picker_area);
    frame.render_widget(
        Paragraph::new(lines).block(modal_block("Add to playlist", theme)),
        picker_area,
    );
}

fn modal_block<'a>(title: &'a str, theme: &theme::Theme) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.border_active)
        .title(format!(" {title} "))
}
