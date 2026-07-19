use crate::app::state::AppState;
use crate::history::HistoryService;
use crate::history::model::{HistoryEntry, PlaybackOutcome};
use crate::media::Track;

use super::{TrackContextAction, TrackSource};

mod dispatch;
mod filtered;
mod input;
mod modal;
mod removals;
mod resolver;

struct ResolverCase {
    name: &'static str,
    state: AppState,
    history: Option<HistoryService>,
    expected_track_title: &'static str,
    expected_source: TrackSource,
    expected_actions: Vec<TrackContextAction>,
}

fn track(id: &str, title: &str) -> Track {
    let mut track = Track::new(id, title, "Channel");
    track.channel_id = Some(format!("channel-{id}"));
    track.channel_url = Some(format!("https://www.youtube.com/channel/channel-{id}"));
    track
}

fn history(tracks: &[Track]) -> HistoryService {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut history =
        HistoryService::load(&temp.path().join("history.json"), 100).expect("history");
    for track in tracks {
        history.record(HistoryEntry::from_track(
            track,
            None,
            PlaybackOutcome::Completed,
            60,
        ));
    }
    history
}

fn standard_actions(include_add_to_queue: bool) -> Vec<TrackContextAction> {
    let mut actions = vec![TrackContextAction::PlayNow, TrackContextAction::PlayNext];
    if include_add_to_queue {
        actions.push(TrackContextAction::AddToQueue);
    }
    actions.extend([
        TrackContextAction::AddToPlaylist,
        TrackContextAction::VisitChannel,
        TrackContextAction::ShowDetails,
        TrackContextAction::OpenInBrowser,
        TrackContextAction::CopyUrl,
    ]);
    actions
}
