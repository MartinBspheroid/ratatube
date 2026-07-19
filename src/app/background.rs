//! Query classification and cancellable yt-dlp background work.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, HistoryAction, NavigationAction, PlaybackAction, PlaylistAction};
use crate::app::operations::OperationKind;
use crate::app::reducer::reduce;
use crate::app::state::Focus;
use crate::media::Track;
use crate::media::import::{InputKind, classify_input};

impl App {
    /// Classify free text vs URL and dispatch accordingly (PRD 10.2).
    pub(super) async fn submit_text_query(
        &mut self,
        query: String,
        action_tx: &mpsc::Sender<Action>,
    ) {
        let kind = classify_input(&query);
        self.state.input_kind = Some(kind.clone());
        match kind {
            InputKind::Query(value) => {
                let _ = action_tx
                    .send(Action::Navigation(NavigationAction::SubmitSearch(value)))
                    .await;
            }
            InputKind::Video(id) => {
                let url = format!("https://www.youtube.com/watch?v={id}");
                self.autoplay_first_search = true;
                let _ = action_tx
                    .send(Action::Navigation(NavigationAction::SubmitExactVideo(url)))
                    .await;
            }
            InputKind::Playlist(_) => {
                let _ = action_tx
                    .send(Action::Playlists(PlaylistAction::StartImport(query)))
                    .await;
                self.state.focus = Focus::Content;
            }
            InputKind::Mix(video_id) => {
                self.state.notify("Loading mix...", false);
                self.state.focus = Focus::Content;
                let yt_dlp = self.yt_dlp.clone();
                let tx = action_tx.clone();
                let ticket = self.operations.start(OperationKind::Mix);
                let operation_id = ticket.id();
                let cancellation = ticket.cancellation().clone();
                let handle = tokio::spawn(async move {
                    let action = tokio::select! {
                        () = cancellation.cancelled() => return,
                        result = yt_dlp.fetch_mix(&video_id) => match result {
                            Ok(fetch) => Action::Playback(PlaybackAction::MixLoaded {
                                operation_id,
                                title: fetch.title,
                                tracks: fetch.tracks,
                            }),
                            Err(err) => Action::History(HistoryAction::Notify(format!("Mix failed: {err}"))),
                        }
                    };
                    let _ = tx.send(action).await;
                });
                self.operations
                    .attach(OperationKind::Mix, operation_id, handle);
            }
        }
    }

    /// Run a cancellable yt-dlp search; completion returns through the action channel.
    pub(super) fn spawn_search(
        &mut self,
        query: String,
        generation: u64,
        action_tx: mpsc::Sender<Action>,
    ) {
        let ticket = self.operations.start(OperationKind::Search);
        let operation_id = ticket.id();
        let yt_dlp = self.yt_dlp.clone();
        let limit = self.config.search.result_limit;
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.search(&query, limit) => match result {
                    Ok(tracks) => Action::Navigation(NavigationAction::SearchCompleted { generation, tracks }),
                    Err(err) => Action::Navigation(NavigationAction::SearchFailed {
                        generation,
                        message: err.to_string(),
                    }),
                }
            };
            let _ = action_tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Search, operation_id, handle);
    }

    /// Fetch one exact video in a cancellable search-domain operation.
    pub(super) fn spawn_exact_video(
        &mut self,
        url: String,
        generation: u64,
        action_tx: mpsc::Sender<Action>,
    ) {
        let ticket = self.operations.start(OperationKind::Search);
        let operation_id = ticket.id();
        let yt_dlp = self.yt_dlp.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_video(&url) => match result {
                    Ok(track) => Action::Navigation(NavigationAction::SearchCompleted {
                        generation,
                        tracks: vec![track],
                    }),
                    Err(err) => Action::Navigation(NavigationAction::SearchFailed {
                        generation,
                        message: err.to_string(),
                    }),
                }
            };
            let _ = action_tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Search, operation_id, handle);
    }

    /// Run a cancellable playlist import; completion returns through the action channel.
    pub(super) fn spawn_import(&mut self, url: String, action_tx: mpsc::Sender<Action>) {
        let ticket = self.operations.start(OperationKind::Import);
        let operation_id = ticket.id();
        let _ = reduce(
            &mut self.state,
            Action::Playlists(PlaylistAction::ImportStarted {
                operation_id,
                url: url.clone(),
            }),
        );
        let yt_dlp = self.yt_dlp.clone();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.fetch_playlist(&url) => match result {
                    Ok(fetch) => Action::Playlists(PlaylistAction::ImportCompleted {
                        operation_id,
                        url,
                        title: fetch.title,
                        remote_id: fetch.remote_id,
                        tracks: fetch.tracks,
                        rejections: fetch.rejections,
                    }),
                    Err(err) => Action::Playlists(PlaylistAction::ImportFailed {
                        operation_id,
                        url,
                        message: err.to_string(),
                    }),
                }
            };
            let _ = action_tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Import, operation_id, handle);
    }

    /// Resolve a persisted resume target through the existing session action flow.
    pub(super) fn spawn_pending_resume_resolution(
        &mut self,
        track: Track,
        action_tx: &mpsc::Sender<Action>,
    ) {
        let yt_dlp = self.yt_dlp.clone();
        let tx = action_tx.clone();
        let ticket = self.operations.start(OperationKind::Session);
        let operation_id = ticket.id();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let action = tokio::select! {
                () = cancellation.cancelled() => return,
                result = yt_dlp.resolve_stream(&track.webpage_url) => match result {
                    Ok(url) => Action::Playback(PlaybackAction::SessionStreamResolved {
                        operation_id,
                        track_id: track.id.clone(),
                        url,
                    }),
                    Err(error) => Action::Playback(PlaybackAction::SessionResolveFailed {
                        operation_id,
                        track_id: track.id.clone(),
                        message: error.to_string(),
                    }),
                }
            };
            let _ = tx.send(action).await;
        });
        self.operations
            .attach(OperationKind::Session, operation_id, handle);
    }
}
