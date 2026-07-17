//! Queue data model: ordering, repeat, shuffle, and history (PRD 10.5-10.6).

use serde::{Deserialize, Serialize};

use crate::media::Track;

/// Repeat behavior (PRD 10.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepeatMode {
    #[default]
    Off,
    Track,
    Queue,
}

impl RepeatMode {
    /// Cycle to the next mode: Off -> Track -> Queue -> Off.
    pub fn next(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::Track,
            RepeatMode::Track => RepeatMode::Queue,
            RepeatMode::Queue => RepeatMode::Off,
        }
    }
}

/// The active ordered list of tracks.
///
/// `order` holds indices into `tracks` in play order; shuffle permutes
/// `order` rather than duplicating tracks. `play_history` records actual
/// played order so Previous works correctly in shuffle mode (PRD 10.6).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Queue {
    pub tracks: Vec<Track>,
    /// Play order as indices into `tracks`.
    pub order: Vec<usize>,
    /// Position within `order` of the current track.
    pub position: Option<usize>,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// Stack of previously played `order` positions.
    #[serde(skip)]
    pub play_history: Vec<usize>,
}

impl Queue {
    /// Append a track to the end of the queue.
    pub fn push(&mut self, track: Track) {
        self.tracks.push(track);
        self.order.push(self.tracks.len() - 1);
    }

    /// Insert a track so it plays immediately after the current one.
    pub fn push_next(&mut self, track: Track) {
        self.tracks.push(track);
        let track_index = self.tracks.len() - 1;
        let insert_at = self.position.map_or(0, |p| p + 1);
        self.order
            .insert(insert_at.min(self.order.len()), track_index);
    }

    /// Remove the track at play-order position `pos`.
    pub fn remove_at(&mut self, pos: usize) -> Option<Track> {
        if pos >= self.order.len() {
            return None;
        }
        let track_index = self.order.remove(pos);
        self.play_history.retain(|&p| p != pos);
        self.position = match self.position {
            Some(p) if p == pos => None,
            Some(p) if p > pos => Some(p - 1),
            other => other,
        };
        Some(self.tracks[track_index].clone())
    }

    /// Move the item at play-order position `from` to position `to`.
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.order.len() || to >= self.order.len() || from == to {
            return;
        }
        let item = self.order.remove(from);
        self.order.insert(to, item);
        self.position = self.position.map(|p| {
            if p == from {
                to
            } else if from < to && p > from && p <= to {
                p - 1
            } else if to < from && p >= to && p < from {
                p + 1
            } else {
                p
            }
        });
    }

    /// Remove all tracks.
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.order.clear();
        self.position = None;
        self.play_history.clear();
    }

    /// The track currently selected, if any.
    pub fn current(&self) -> Option<&Track> {
        let pos = self.position?;
        self.order.get(pos).map(|&i| &self.tracks[i])
    }

    /// Replace contents with `tracks`, preserving order (identity order).
    pub fn load_tracks(&mut self, tracks: Vec<Track>) {
        self.clear();
        self.order = (0..tracks.len()).collect();
        self.tracks = tracks;
    }

    /// Toggle shuffle; reshuffles the remaining order when enabled and
    /// restores identity order when disabled.
    pub fn set_shuffle(&mut self, shuffle: bool) {
        if self.shuffle == shuffle {
            return;
        }
        self.shuffle = shuffle;
        if shuffle {
            self.shuffle_remaining();
        } else {
            let current_track = self.position.map(|p| self.order[p]);
            self.order = (0..self.tracks.len()).collect();
            self.position = current_track.and_then(|t| self.order.iter().position(|&x| x == t));
        }
    }

    fn shuffle_remaining(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        match self.position {
            // Keep the current track first, shuffle the rest.
            Some(p) if !self.order.is_empty() => {
                self.order.swap(0, p);
                self.position = Some(0);
                self.order[1..].shuffle(&mut rng);
            }
            _ => self.order.shuffle(&mut rng),
        }
    }

    /// Advance to the next track, honoring repeat mode.
    /// Returns the track to play, or `None` when the queue is exhausted.
    pub fn advance(&mut self) -> Option<&Track> {
        if self.repeat == RepeatMode::Track {
            return self.current();
        }
        let next_pos = match self.position {
            None => 0,
            Some(p) => {
                self.play_history.push(p);
                p + 1
            }
        };
        if next_pos >= self.order.len() {
            if self.repeat == RepeatMode::Queue && !self.order.is_empty() {
                self.position = Some(0);
            } else {
                self.position = None;
                return None;
            }
        } else {
            self.position = Some(next_pos);
        }
        self.current()
    }

    /// Previous-track behavior (PRD 10.6): restart when past `restart_threshold`
    /// seconds, otherwise use actual play history.
    pub fn previous(&mut self, position_seconds: u64, restart_threshold: u64) -> PreviousOutcome {
        if position_seconds > restart_threshold && self.current().is_some() {
            return PreviousOutcome::RestartCurrent;
        }
        if let Some(prev) = self.play_history.pop() {
            self.position = Some(prev);
            return PreviousOutcome::PlayPrevious;
        }
        match self.position {
            Some(p) if p > 0 => {
                self.position = Some(p - 1);
                PreviousOutcome::PlayPrevious
            }
            _ => PreviousOutcome::RestartCurrent,
        }
    }
}

/// What the controller should do after a Previous action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviousOutcome {
    RestartCurrent,
    PlayPrevious,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str) -> Track {
        Track::new(id, id, "artist")
    }

    fn queue_with(ids: &[&str]) -> Queue {
        let mut q = Queue::default();
        for id in ids {
            q.push(track(id));
        }
        q
    }

    #[test]
    fn push_next_inserts_after_current() {
        let mut q = queue_with(&["a", "b", "c"]);
        q.position = Some(0);
        q.push_next(track("x"));
        let order_ids: Vec<&str> = q.order.iter().map(|&i| q.tracks[i].id.as_str()).collect();
        assert_eq!(order_ids, vec!["a", "x", "b", "c"]);
    }

    #[test]
    fn advance_stops_at_end_when_repeat_off() {
        let mut q = queue_with(&["a", "b"]);
        assert_eq!(q.advance().map(|t| t.id.as_str()), Some("a"));
        assert_eq!(q.advance().map(|t| t.id.as_str()), Some("b"));
        assert!(q.advance().is_none());
        assert_eq!(q.position, None);
    }

    #[test]
    fn repeat_track_stays() {
        let mut q = queue_with(&["a", "b"]);
        q.repeat = RepeatMode::Track;
        q.position = Some(0);
        assert_eq!(q.advance().map(|t| t.id.as_str()), Some("a"));
    }

    #[test]
    fn repeat_queue_wraps() {
        let mut q = queue_with(&["a", "b"]);
        q.repeat = RepeatMode::Queue;
        q.position = Some(1);
        assert_eq!(q.advance().map(|t| t.id.as_str()), Some("a"));
    }

    #[test]
    fn previous_restarts_when_past_threshold() {
        let mut q = queue_with(&["a", "b"]);
        q.position = Some(1);
        assert_eq!(q.previous(10, 5), PreviousOutcome::RestartCurrent);
    }

    #[test]
    fn previous_uses_play_history_in_shuffle() {
        let mut q = queue_with(&["a", "b", "c"]);
        q.set_shuffle(true);
        q.position = Some(0);
        let first = q.current().expect("current").id.clone();
        q.advance();
        let second = q.current().expect("current").id.clone();
        assert_ne!(first, second);
        assert_eq!(q.previous(2, 5), PreviousOutcome::PlayPrevious);
        assert_eq!(q.current().expect("current").id, first);
    }

    #[test]
    fn reorder_keeps_current_selection() {
        let mut q = queue_with(&["a", "b", "c"]);
        q.position = Some(2); // "c"
        q.reorder(0, 1);
        assert_eq!(q.current().expect("current").id, "c");
        assert_eq!(q.position, Some(2));
    }

    #[test]
    fn unshuffle_restores_identity_order() {
        let mut q = queue_with(&["a", "b", "c", "d"]);
        q.position = Some(0);
        q.set_shuffle(true);
        let current = q.current().expect("current").id.clone();
        q.set_shuffle(false);
        let ids: Vec<&str> = q.order.iter().map(|&i| q.tracks[i].id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c", "d"]);
        assert_eq!(q.current().expect("current").id, current);
    }
}
