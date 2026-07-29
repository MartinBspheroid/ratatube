//! Queue mutation and cursor movement operations.

use rand::seq::SliceRandom;

use super::{PreviousOutcome, Queue, RepeatMode};
use crate::media::Track;

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
        let insert_at = self
            .position
            .map_or(0, |position| position + 1)
            .min(self.order.len());
        self.order.insert(insert_at, track_index);
        self.absorb_insertion(insert_at);
    }

    /// Insert a track at a play-order position.
    pub fn insert_at(&mut self, position: usize, track: Track) {
        let track_index = self.tracks.len();
        self.tracks.push(track);
        let insert_at = position.min(self.order.len());
        self.order.insert(insert_at, track_index);
        self.absorb_insertion(insert_at);
    }

    /// Remove the track at play-order position `position`.
    pub fn remove_at(&mut self, position: usize) -> Option<Track> {
        let track_index = *self.order.get(position)?;
        self.order.remove(position);
        let track = self.tracks.remove(track_index);
        for index in &mut self.order {
            if *index > track_index {
                *index -= 1;
            }
        }
        self.play_history = self
            .play_history
            .iter()
            .filter_map(|&history| match history.cmp(&position) {
                std::cmp::Ordering::Less => Some(history),
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => Some(history - 1),
            })
            .collect();
        self.position = match self.position {
            Some(current) if current == position => None,
            Some(current) if current > position => Some(current - 1),
            current => current,
        };
        Some(track)
    }

    /// Move the item at play-order position `from` to position `to`.
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.order.len() || to >= self.order.len() || from == to {
            return;
        }
        let item = self.order.remove(from);
        self.order.insert(to, item);
        self.absorb_move(from, to);
    }

    /// Shift every stored play-order position to absorb an insertion.
    ///
    /// `position` and `play_history` both index `order`, so any structural
    /// change has to move them together or Previous lands on the wrong track.
    fn absorb_insertion(&mut self, insert_at: usize) {
        if let Some(current) = &mut self.position
            && *current >= insert_at
        {
            *current += 1;
        }
        for history_position in &mut self.play_history {
            if *history_position >= insert_at {
                *history_position += 1;
            }
        }
    }

    /// Shift every stored play-order position to follow a move.
    fn absorb_move(&mut self, from: usize, to: usize) {
        self.position = self.position.map(|position| moved(position, from, to));
        for history_position in &mut self.play_history {
            *history_position = moved(*history_position, from, to);
        }
    }

    /// Remove all tracks.
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.order.clear();
        self.position = None;
        self.play_history.clear();
    }

    /// Replace contents with `tracks`, preserving identity order.
    pub fn load_tracks(&mut self, tracks: Vec<Track>) {
        self.clear();
        self.order = (0..tracks.len()).collect();
        self.tracks = tracks;
    }

    /// Toggle shuffle without changing the current track.
    pub fn set_shuffle(&mut self, shuffle: bool) {
        if self.shuffle == shuffle {
            return;
        }
        self.shuffle = shuffle;
        if shuffle {
            self.shuffle_remaining();
        } else {
            let current_track = self.position.map(|position| self.order[position]);
            self.order = (0..self.tracks.len()).collect();
            self.position =
                current_track.and_then(|track| self.order.iter().position(|&x| x == track));
        }
    }

    fn shuffle_remaining(&mut self) {
        let mut rng = rand::rng();
        match self.position {
            Some(position) if !self.order.is_empty() => {
                self.order.swap(0, position);
                self.position = Some(0);
                self.order[1..].shuffle(&mut rng);
            }
            _ => self.order.shuffle(&mut rng),
        }
    }

    /// Advance to the next track while honoring repeat mode.
    pub fn advance(&mut self) -> Option<&Track> {
        if self.repeat == RepeatMode::Track {
            return self.current();
        }
        let next_position = match self.position {
            None => 0,
            Some(position) => {
                self.play_history.push(position);
                position + 1
            }
        };
        if next_position >= self.order.len() {
            if self.repeat == RepeatMode::Queue && !self.order.is_empty() {
                self.position = Some(0);
            } else {
                self.position = None;
                return None;
            }
        } else {
            self.position = Some(next_position);
        }
        self.current()
    }

    /// Resolve Previous as restart-current or actual prior play-order entry.
    pub fn previous(&mut self, position_seconds: u64, restart_threshold: u64) -> PreviousOutcome {
        if position_seconds > restart_threshold && self.current().is_some() {
            return PreviousOutcome::RestartCurrent;
        }
        if let Some(previous) = self.play_history.pop() {
            self.position = Some(previous);
            return PreviousOutcome::PlayPrevious;
        }
        match self.position {
            Some(position) if position > 0 => {
                self.position = Some(position - 1);
                PreviousOutcome::PlayPrevious
            }
            _ => PreviousOutcome::RestartCurrent,
        }
    }
}

/// Where the play-order position `position` ends up once the item at `from`
/// moves to `to`.
fn moved(position: usize, from: usize, to: usize) -> usize {
    if position == from {
        to
    } else if from < to && position > from && position <= to {
        position - 1
    } else if to < from && position >= to && position < from {
        position + 1
    } else {
        position
    }
}
