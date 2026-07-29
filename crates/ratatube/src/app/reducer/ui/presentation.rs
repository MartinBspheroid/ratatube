//! Presentation-mode toggles and notification transitions.

use crate::app::action::HistoryAction;
use crate::app::actions::UiMsg;
use crate::app::reducer::Effect;
use crate::app::state::{DomainState, HistoryViewMode, PlayingPane, UiState, View};

/// Reduce service-notification transitions that remain history-family.
pub(in crate::app::reducer) fn reduce_history(
    ui: &mut UiState,
    action: HistoryAction,
) -> Vec<Effect> {
    if let HistoryAction::Notify(message) = action {
        ui.notify(&message, false);
    }
    Vec::new()
}

/// Reduce pane, view-mode, and notification presentation messages.
pub(in crate::app::reducer) fn reduce(
    ui: &mut UiState,
    domain: &DomainState,
    msg: UiMsg,
) -> Vec<Effect> {
    match msg {
        UiMsg::ToggleNotificationLog => {
            ui.show_notification_log = !ui.show_notification_log;
        }
        UiMsg::ToggleHistoryViewMode => {
            ui.history_view_mode = match ui.history_view_mode {
                HistoryViewMode::Recent => HistoryViewMode::Top,
                HistoryViewMode::Top => HistoryViewMode::Recent,
            };
            ui.selected_index = 0;
            ui.reset_list(domain);
        }
        UiMsg::DismissNotification => ui.notification = None,
        UiMsg::ScrollNowPlaying(delta) => {
            let next = i32::from(ui.now_playing_scroll) + delta;
            ui.now_playing_scroll = next.max(0) as u16;
        }
        UiMsg::ToggleNowPlayingPane => {
            ui.now_playing_show_description = !ui.now_playing_show_description;
            ui.now_playing_scroll = 0;
        }
        UiMsg::CyclePlayingPane => {
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
