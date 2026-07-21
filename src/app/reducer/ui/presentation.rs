//! Presentation-mode toggles and notification transitions.

use crate::app::action::{HistoryAction, PlaybackAction};
use crate::app::reducer::Effect;
use crate::app::state::{DomainState, HistoryViewMode, PlayingPane, UiState, View};

/// Reduce UI-only history-family transitions (view modes, notifications).
pub(in crate::app::reducer) fn reduce_history(
    ui: &mut UiState,
    domain: &DomainState,
    action: HistoryAction,
) -> Vec<Effect> {
    match action {
        HistoryAction::ToggleNotificationLog => {
            ui.show_notification_log = !ui.show_notification_log;
        }
        HistoryAction::ToggleHistoryViewMode => {
            ui.history_view_mode = match ui.history_view_mode {
                HistoryViewMode::Recent => HistoryViewMode::Top,
                HistoryViewMode::Top => HistoryViewMode::Recent,
            };
            ui.selected_index = 0;
            ui.reset_list(domain);
        }
        HistoryAction::Notify(message) => ui.notify(&message, false),
        HistoryAction::DismissNotification => ui.notification = None,
        _ => {}
    }
    Vec::new()
}

/// Reduce UI-only Playing-view pane and scroll transitions.
pub(in crate::app::reducer) fn reduce_playing_panes(
    ui: &mut UiState,
    domain: &DomainState,
    action: PlaybackAction,
) -> Vec<Effect> {
    match action {
        PlaybackAction::ScrollNowPlaying(delta) => {
            let next = i32::from(ui.now_playing_scroll) + delta;
            ui.now_playing_scroll = next.max(0) as u16;
        }
        PlaybackAction::ToggleNowPlayingPane => {
            ui.now_playing_show_description = !ui.now_playing_show_description;
            ui.now_playing_scroll = 0;
        }
        PlaybackAction::CyclePlayingPane => {
            if ui.view == View::NowPlaying
                && crate::ui::layout::Breakpoint::from_width(ui.screen_area.width)
                    == crate::ui::layout::Breakpoint::UltraWide
            {
                ui.playing_pane = match ui.playing_pane {
                    PlayingPane::Info => PlayingPane::Queue,
                    PlayingPane::Queue => PlayingPane::Info,
                };
                if ui.playing_pane == PlayingPane::Queue {
                    ui.selected_index = domain.queue.position.unwrap_or(0);
                }
                ui.reset_list(domain);
            }
        }
        _ => {}
    }
    Vec::new()
}
