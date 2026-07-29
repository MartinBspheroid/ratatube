//! Client-side domain mirror: a [`DomainState`] hydrated from the daemon's
//! snapshot and kept fresh by broadcast events, so all Phase 1 rendering
//! code works unchanged. Search and channel fields stay client-owned
//! (per-client request/reply per the design spec) and are never overwritten
//! by broadcasts.

use crate::app::state::DomainState;
use crate::protocol::{Snapshot, WireEvent};
use crate::queue::Queue;

/// Install a queue that arrived over the wire, or refuse it.
///
/// The daemon is external data: the socket path is predictable and a peer can
/// be version-skewed, buggy or hostile. Renderers and `Queue::set_shuffle`
/// index `tracks` by `order` entries unguarded, so an out-of-range entry would
/// panic the client's render loop. This is the same invariant the on-disk path
/// enforces in `queue/service.rs::load`, checked with the same
/// [`Queue::validate`] so the two ingress points cannot drift.
///
/// The revision is deliberately *not* advanced for a rejected queue: it is the
/// version tag of the queue the mirror actually holds, and the client sends it
/// back as `expected_revision` on queue commands. Accepting the revision while
/// keeping the old queue would make the mirror claim to be fresher than it is
/// and let the daemon accept an index-based command computed against content
/// the client never received.
fn install_queue(domain: &mut DomainState, queue: Queue, queue_revision: u64) {
    if let Err(error) = queue.validate() {
        tracing::warn!(
            ?error,
            queue_revision,
            "invalid queue from daemon; keeping the mirrored queue and revision"
        );
        return;
    }
    domain.queue = queue;
    domain.queue_revision = queue_revision;
}

/// Replace the mirrored daemon-owned fields from a full snapshot.
pub fn apply_snapshot(domain: &mut DomainState, snapshot: Snapshot) {
    let Snapshot {
        queue,
        queue_revision,
        playback,
        current_track,
        current_details,
        playlists,
        playlists_revision,
        health,
    } = snapshot;
    install_queue(domain, queue, queue_revision);
    domain.playback = playback;
    domain.current_track = current_track;
    domain.current_details = current_details;
    domain.playlists = playlists;
    domain.playlists_revision = playlists_revision;
    domain.mpv_ready = health.mpv_ready;
    domain.yt_dlp_ready = health.yt_dlp_ready;
}

/// Apply one broadcast event to the mirror.
pub fn apply_event(domain: &mut DomainState, event: WireEvent) {
    match event {
        WireEvent::QueueChanged {
            queue,
            queue_revision,
        } => install_queue(domain, queue, queue_revision),
        WireEvent::PlaybackProgress { playback } => domain.playback = playback,
        WireEvent::TrackChanged { track } => {
            // Mirrors the daemon's track switch: details for the previous
            // track are stale until a TrackDetailsChanged arrives.
            domain.current_track = track;
            domain.current_details = None;
        }
        WireEvent::TrackDetailsChanged { details } => domain.current_details = details,
        WireEvent::PlaylistsChanged {
            playlists,
            playlists_revision,
        } => {
            domain.playlists = playlists;
            domain.playlists_revision = playlists_revision;
        }
        WireEvent::Health { health } => {
            domain.mpv_ready = health.mpv_ready;
            domain.yt_dlp_ready = health.yt_dlp_ready;
        }
        WireEvent::ImportChanged { import } => {
            domain.import = import.map(crate::protocol::WireImport::into_state);
        }
        // History is reloaded from the shared on-disk store by the client
        // runtime; channel state needs the runtime's navigation stack and is
        // applied there.
        WireEvent::HistoryChanged | WireEvent::ChannelChanged { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Track;
    use crate::protocol::Health;

    fn snapshot_with_track() -> Snapshot {
        let mut domain = DomainState::default();
        domain.queue.push(Track::new("a", "Title A", "Channel"));
        domain.current_track = Some(Track::new("a", "Title A", "Channel"));
        domain.mpv_ready = true;
        Snapshot::from(&domain)
    }

    /// A queue whose single `order` entry points past the end of `tracks`;
    /// rendering and `set_shuffle` would panic indexing on it.
    fn queue_with_out_of_range_order() -> Queue {
        let mut queue = Queue::default();
        queue.push(Track::new("z", "Title Z", "Channel"));
        queue.order = vec![9];
        queue
    }

    /// A mirror already holding one valid track at a known revision.
    fn hydrated_mirror() -> DomainState {
        let mut mirror = DomainState::default();
        apply_event(
            &mut mirror,
            WireEvent::QueueChanged {
                queue: {
                    let mut queue = Queue::default();
                    queue.push(Track::new("a", "Title A", "Channel"));
                    queue
                },
                queue_revision: 3,
            },
        );
        mirror
    }

    #[test]
    fn snapshot_hydrates_the_mirror() {
        let mut mirror = DomainState::default();
        apply_snapshot(&mut mirror, snapshot_with_track());
        assert_eq!(mirror.queue.tracks.len(), 1);
        assert_eq!(
            mirror.current_track.as_ref().map(|t| t.id.as_str()),
            Some("a")
        );
        assert!(mirror.mpv_ready);
    }

    #[test]
    fn queue_event_replaces_queue_and_revision() {
        let mut mirror = DomainState::default();
        let mut queue = crate::queue::Queue::default();
        queue.push(Track::new("b", "Title B", "Channel"));
        apply_event(
            &mut mirror,
            WireEvent::QueueChanged {
                queue,
                queue_revision: 7,
            },
        );
        assert_eq!(mirror.queue.tracks.len(), 1);
        assert_eq!(mirror.queue_revision, 7);
    }

    #[test]
    fn snapshot_with_invalid_queue_keeps_the_mirrored_queue() {
        let mut mirror = hydrated_mirror();
        let daemon = DomainState {
            queue: queue_with_out_of_range_order(),
            queue_revision: 42,
            ..Default::default()
        };

        apply_snapshot(&mut mirror, Snapshot::from(&daemon));

        assert_eq!(
            mirror
                .queue
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a"]
        );
        assert_eq!(mirror.queue.order, vec![0]);
        assert_eq!(mirror.queue_revision, 3);
    }

    #[test]
    fn queue_event_with_invalid_queue_keeps_the_mirrored_queue() {
        let mut mirror = hydrated_mirror();

        apply_event(
            &mut mirror,
            WireEvent::QueueChanged {
                queue: queue_with_out_of_range_order(),
                queue_revision: 42,
            },
        );

        assert_eq!(
            mirror
                .queue
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a"]
        );
        assert_eq!(mirror.queue_revision, 3);
    }

    #[test]
    fn a_valid_queue_still_applies_after_a_rejection() {
        let mut mirror = hydrated_mirror();
        apply_event(
            &mut mirror,
            WireEvent::QueueChanged {
                queue: queue_with_out_of_range_order(),
                queue_revision: 42,
            },
        );

        let mut queue = Queue::default();
        queue.push(Track::new("b", "Title B", "Channel"));
        queue.push(Track::new("c", "Title C", "Channel"));
        apply_event(
            &mut mirror,
            WireEvent::QueueChanged {
                queue,
                queue_revision: 43,
            },
        );

        assert_eq!(
            mirror
                .queue
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "c"]
        );
        assert_eq!(mirror.queue_revision, 43);
    }

    #[test]
    fn track_change_clears_stale_details() {
        let mut mirror = DomainState {
            current_details: Some(crate::media::TrackDetails::default()),
            ..Default::default()
        };
        apply_event(
            &mut mirror,
            WireEvent::TrackChanged {
                track: Some(Track::new("c", "Title C", "Channel")),
            },
        );
        assert_eq!(
            mirror.current_track.as_ref().map(|t| t.id.as_str()),
            Some("c")
        );
        assert!(mirror.current_details.is_none());
    }

    #[test]
    fn events_never_touch_client_owned_search_state() {
        let mut mirror = DomainState {
            search: crate::media::search::SearchState::Results {
                query: "local".into(),
                tracks: vec![Track::new("s", "Search hit", "Channel")],
            },
            ..Default::default()
        };
        apply_event(
            &mut mirror,
            WireEvent::Health {
                health: Health {
                    mpv_ready: true,
                    yt_dlp_ready: true,
                },
            },
        );
        assert!(matches!(
            mirror.search,
            crate::media::search::SearchState::Results { .. }
        ));
    }
}
