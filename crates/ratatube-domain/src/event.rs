//! Broadcast-shaped domain change notifications.
//!
//! The daemon derives these from state counters and sends them to every
//! attached client; the client mirror is the only consumer.

/// One coarse-grained domain change, named for the state it invalidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainEvent {
    QueueChanged,
    PlaybackChanged,
    TrackChanged,
    TrackDetailsChanged,
    PlaylistsChanged,
    HistoryChanged,
    SearchChanged,
    ChannelChanged,
    ImportChanged,
    Health,
}
