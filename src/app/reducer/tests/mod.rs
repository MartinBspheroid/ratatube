use super::{Effect, reduce};
use crate::app::action::{
    Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction, QueueAction,
};
use crate::app::operations::{OperationKind, OperationRegistry};
use crate::app::state::{
    AppState, DetailsStatus, Notification, OperationStatus, PlayingPane, View,
};
use crate::media::Track;
use crate::media::search::SearchState;
use crate::playback::PlaybackEvent;

fn track(id: &str) -> Track {
    Track::new(id, id, "artist")
}

fn playback_event(event: PlaybackEvent) -> Action {
    Action::Playback(PlaybackAction::PlaybackEvent(event))
}

mod daemon_commands;
mod domain_events;
mod navigation_queue;
mod playback_core;
mod playback_features;
mod playback_transition;
mod playback_transition_races;
mod playlists;
