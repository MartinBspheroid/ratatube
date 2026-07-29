//! Queue data model: ordering, repeat, shuffle, and history (PRD 10.5-10.6).

use serde::{Deserialize, Serialize};

use crate::media::Track;

mod operations;

/// Repeat behavior (PRD 10.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepeatMode {
    #[default]
    Off,
    Track,
    Queue,
}

/// Structural error found while validating queue indices and selection.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueueValidationError {
    #[error("play order has {order_len} entries for {track_len} tracks")]
    OrderLength { order_len: usize, track_len: usize },
    #[error("play-order index {index} is outside {track_len} tracks")]
    OrderIndex { index: usize, track_len: usize },
    #[error("play-order index {0} occurs more than once")]
    DuplicateOrderIndex(usize),
    #[error("queue position {position} is outside {order_len} play-order entries")]
    Position { position: usize, order_len: usize },
    #[error("play-history position {position} is outside {order_len} play-order entries")]
    PlayHistory { position: usize, order_len: usize },
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
    /// Validate all cross-field queue invariants before runtime use.
    pub fn validate(&self) -> Result<(), QueueValidationError> {
        if self.order.len() != self.tracks.len() {
            return Err(QueueValidationError::OrderLength {
                order_len: self.order.len(),
                track_len: self.tracks.len(),
            });
        }

        let mut seen = vec![false; self.tracks.len()];
        for &index in &self.order {
            let Some(slot) = seen.get_mut(index) else {
                return Err(QueueValidationError::OrderIndex {
                    index,
                    track_len: self.tracks.len(),
                });
            };
            if *slot {
                return Err(QueueValidationError::DuplicateOrderIndex(index));
            }
            *slot = true;
        }

        if let Some(position) = self.position
            && position >= self.order.len()
        {
            return Err(QueueValidationError::Position {
                position,
                order_len: self.order.len(),
            });
        }
        if let Some(&position) = self
            .play_history
            .iter()
            .find(|&&position| position >= self.order.len())
        {
            return Err(QueueValidationError::PlayHistory {
                position,
                order_len: self.order.len(),
            });
        }
        Ok(())
    }

    /// The track currently selected, if any.
    pub fn current(&self) -> Option<&Track> {
        let pos = self.position?;
        self.order
            .get(pos)
            .and_then(|&index| self.tracks.get(index))
    }

    /// Return the track that would play next without changing queue state.
    pub fn effective_next(&self) -> Option<&Track> {
        if self.repeat == RepeatMode::Track {
            return None;
        }
        let next_position = match self.position {
            Some(position)
                if position
                    .checked_add(1)
                    .is_some_and(|next| next < self.order.len()) =>
            {
                position + 1
            }
            Some(position) if position < self.order.len() && self.repeat == RepeatMode::Queue => 0,
            None if !self.order.is_empty() => 0,
            _ => return None,
        };
        self.order
            .get(next_position)
            .and_then(|&track_index| self.tracks.get(track_index))
    }
}

/// What the controller should do after a Previous action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviousOutcome {
    RestartCurrent,
    PlayPrevious,
}

#[cfg(test)]
mod tests;
