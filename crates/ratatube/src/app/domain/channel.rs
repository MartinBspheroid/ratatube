//! Channel-browser data flow: bounded page fetching and Back restoration.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, NavigationAction};
use crate::app::operations::OperationKind;
use crate::app::state::{ChannelNavigationSnapshot, ChannelState, Focus, View};
use crate::media::Track;
use crate::media::channel::{ChannelPage, ChannelPageRequest};

impl App {
    pub(in crate::app) fn visit_channel(&mut self, track: Track, action_tx: mpsc::Sender<Action>) {
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

    pub(in crate::app) fn finish_channel_resolve(
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
            view: self.state.ui.view,
            focus: self.state.ui.focus,
            selected_index: self.state.ui.selected_index,
        };
        let previous = self.state.domain.channel.take().map(|mut previous| {
            // Opening a nested channel supersedes its page operation. Make the
            // restored page actionable instead of leaving a stale spinner.
            previous.loading = false;
            Box::new(previous)
        });
        let mut channel = ChannelState::new(&track, url, return_to);
        channel.previous = previous;
        self.state.domain.channel = Some(channel);
        self.state.ui.view = View::Channel;
        self.state.ui.focus = Focus::Content;
        self.state.ui.selected_index = 0;
        self.state.reset_list();
        self.spawn_channel_page(0, action_tx);
    }

    pub(in crate::app) fn spawn_channel_page(
        &mut self,
        page: usize,
        action_tx: mpsc::Sender<Action>,
    ) {
        let Some(channel) = self.state.domain.channel.as_mut() else {
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

    pub(in crate::app) fn finish_channel_page(
        &mut self,
        channel_url: &str,
        page: usize,
        result: std::result::Result<ChannelPage, String>,
    ) {
        let Some(channel) = self.state.domain.channel.as_mut() else {
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

    pub(in crate::app) fn leave_channel(&mut self) {
        self.operations.cancel(OperationKind::ChannelResolve);
        self.operations.cancel(OperationKind::ChannelPage);
        if let Some(mut channel) = self.state.domain.channel.take() {
            if let Some(previous) = channel.previous.take() {
                self.state.domain.channel = Some(*previous);
                self.state.ui.view = View::Channel;
                self.state.ui.focus = channel.return_to.focus;
                self.state.reset_list();
                self.state.ui.selected_index = channel.return_to.selected_index;
                return;
            }
            self.state.ui.view = channel.return_to.view;
            self.state.ui.focus = channel.return_to.focus;
            self.state.reset_list();
            self.state.ui.selected_index = channel.return_to.selected_index;
        }
    }
}

#[cfg(test)]
#[path = "channel/tests.rs"]
mod tests;
