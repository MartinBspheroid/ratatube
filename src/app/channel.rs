//! Dedicated channel-browser state and cancellable data flow.

use std::collections::HashSet;

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction};
use crate::app::operations::OperationKind;
use crate::app::state::{Focus, View};
use crate::media::Track;
use crate::media::channel::{ChannelPage, ChannelPageRequest};

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
}

impl ChannelState {
    fn new(track: &Track, url: String, return_to: ChannelNavigationSnapshot) -> Self {
        Self {
            name: track.artist.clone(),
            url,
            tracks: Vec::new(),
            next_page: 0,
            exhausted: false,
            loading: false,
            error: None,
            return_to,
        }
    }

    fn append(&mut self, page: ChannelPage) {
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

impl App {
    pub(super) fn visit_channel(&mut self, track: Track, action_tx: mpsc::Sender<Action>) {
        if let Some(url) = track.channel_url.clone() {
            self.open_channel(track, url, action_tx);
            return;
        }
        let ticket = self.operations.start(OperationKind::ChannelResolve);
        let operation_id = ticket.id();
        let cancellation = ticket.cancellation().clone();
        let yt_dlp = self.yt_dlp.clone();
        let source_track_id = track.id.clone();
        let webpage_url = track.webpage_url;
        let sender = action_tx;
        let handle = tokio::spawn(async move {
            let result = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_video(&webpage_url) => result.map_err(|error| error.to_string()),
            };
            let _ = sender
                .send(Action::Navigation(NavigationAction::ChannelResolved {
                    operation_id,
                    source_track_id,
                    result,
                }))
                .await;
        });
        self.operations
            .attach(OperationKind::ChannelResolve, operation_id, handle);
    }

    pub(super) fn finish_channel_resolve(
        &mut self,
        result: std::result::Result<Track, String>,
        action_tx: mpsc::Sender<Action>,
    ) {
        match result {
            Ok(track) => match track.channel_url.clone() {
                Some(url) => self.open_channel(track, url, action_tx),
                None => self
                    .state
                    .notify("Channel unavailable for this track", true),
            },
            Err(message) => self
                .state
                .notify(&format!("Channel lookup failed: {message}"), true),
        }
    }

    fn open_channel(&mut self, track: Track, url: String, action_tx: mpsc::Sender<Action>) {
        let request = ChannelPageRequest {
            channel_url: url,
            page: 0,
        };
        let Ok(url) = request.videos_url() else {
            self.state
                .notify("Channel unavailable for this track", true);
            return;
        };
        let return_to = ChannelNavigationSnapshot {
            view: self.state.view,
            focus: self.state.focus,
            selected_index: self.state.selected_index,
        };
        self.state.channel = Some(ChannelState::new(&track, url, return_to));
        self.state.view = View::Channel;
        self.state.focus = Focus::Content;
        self.state.selected_index = 0;
        self.state.reset_list();
        self.spawn_channel_page(0, action_tx);
    }

    pub(super) fn spawn_channel_page(&mut self, page: usize, action_tx: mpsc::Sender<Action>) {
        let Some(channel) = self.state.channel.as_mut() else {
            return;
        };
        if channel.loading || channel.exhausted {
            return;
        }
        channel.loading = true;
        channel.error = None;
        let channel_url = channel.url.clone();
        let request = ChannelPageRequest {
            channel_url: channel_url.clone(),
            page,
        };
        let ticket = self.operations.start(OperationKind::ChannelPage);
        let operation_id = ticket.id();
        let cancellation = ticket.cancellation().clone();
        let yt_dlp = self.yt_dlp.clone();
        let handle = tokio::spawn(async move {
            let result = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_channel_page(&request) => result.map_err(|error| error.to_string()),
            };
            let _ = action_tx
                .send(Action::Navigation(NavigationAction::ChannelPageLoaded {
                    operation_id,
                    channel_url,
                    page,
                    result,
                }))
                .await;
        });
        self.operations
            .attach(OperationKind::ChannelPage, operation_id, handle);
    }

    pub(super) fn finish_channel_page(
        &mut self,
        channel_url: &str,
        page: usize,
        result: std::result::Result<ChannelPage, String>,
    ) {
        let Some(channel) = self.state.channel.as_mut() else {
            return;
        };
        if channel.url != channel_url || channel.next_page != page {
            return;
        }
        match result {
            Ok(page) => channel.append(page),
            Err(message) => {
                channel.loading = false;
                channel.error = Some(message);
            }
        }
        self.state.clamp_selection();
    }

    pub(super) fn leave_channel(&mut self) {
        self.operations.cancel(OperationKind::ChannelResolve);
        self.operations.cancel(OperationKind::ChannelPage);
        if let Some(channel) = self.state.channel.take() {
            self.state.view = channel.return_to.view;
            self.state.focus = channel.return_to.focus;
            self.state.reset_list();
            self.state.selected_index = channel.return_to.selected_index;
        }
    }
}

#[cfg(test)]
#[path = "channel/tests.rs"]
mod tests;
