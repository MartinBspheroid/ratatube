//! Dedicated channel-browser state.

use std::collections::HashSet;

use crate::media::Track;
use crate::media::channel::ChannelPage;
use crate::state::{Focus, View};

/// Navigation values restored when leaving the dedicated channel view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelNavigationSnapshot {
    /// View that opened the channel browser.
    pub view: View,
    /// Focus restored when returning.
    pub focus: Focus,
    /// Selected row restored when returning.
    pub selected_index: usize,
}

/// State owned exclusively by the dedicated channel browser.
#[derive(Debug, Clone)]
pub struct ChannelState {
    /// Display name copied from the source track metadata.
    pub name: String,
    /// Validated canonical YouTube channel videos URL.
    pub url: String,
    /// Loaded videos in newest-first source order.
    pub tracks: Vec<Track>,
    /// Zero-based page retained for the next request or retry.
    pub next_page: usize,
    /// Whether the source has no additional bounded page.
    pub exhausted: bool,
    /// Whether a page request currently owns the loading state.
    pub loading: bool,
    /// Last page failure; existing tracks remain usable while present.
    pub error: Option<String>,
    /// Navigation state restored by Back.
    pub return_to: ChannelNavigationSnapshot,
    /// Previous channel browser restored when navigating between channels.
    pub previous: Option<Box<ChannelState>>,
}

impl ChannelState {
    pub fn new(track: &Track, url: String, return_to: ChannelNavigationSnapshot) -> Self {
        Self {
            name: track.artist.clone(),
            url,
            tracks: Vec::new(),
            next_page: 0,
            exhausted: false,
            loading: false,
            error: None,
            return_to,
            previous: None,
        }
    }

    pub fn append(&mut self, page: ChannelPage) {
        let mut seen: HashSet<String> = self.tracks.iter().map(|track| track.id.clone()).collect();
        self.tracks.extend(
            page.tracks
                .into_iter()
                .filter(|track| seen.insert(track.id.clone())),
        );
        self.exhausted = page.exhausted;
        self.next_page = self.next_page.saturating_add(1);
        self.loading = false;
        self.error = None;
    }
}
