//! Detached, bounded browser and clipboard operations.

use tokio::sync::mpsc;

use crate::app::App;
use crate::app::action::{Action, ExternalCommandKind, ExternalCommandTarget, NavigationAction};
use crate::app::operations::OperationKind;

impl App {
    /// Start one owned external command and return immediately to the event loop.
    pub(super) fn spawn_external_command(
        &mut self,
        command: ExternalCommandKind,
        target: ExternalCommandTarget,
        url: String,
        action_tx: mpsc::Sender<Action>,
    ) {
        let ticket = self.operations.start(OperationKind::ExternalCommand);
        let operation_id = ticket.id();
        let cancellation = ticket.cancellation().clone();
        let handle = tokio::spawn(async move {
            let result = run_external_command(command, &url, &cancellation)
                .await
                .map_err(|error| error.to_string());
            let _ = action_tx
                .send(Action::Navigation(
                    NavigationAction::ExternalCommandCompleted {
                        operation_id,
                        command,
                        target,
                        result,
                    },
                ))
                .await;
        });
        self.operations
            .attach(OperationKind::ExternalCommand, operation_id, handle);
    }

    /// Apply a current external-command result without reopening dismissed UI.
    pub(super) fn finish_external_command(
        &mut self,
        command: ExternalCommandKind,
        target: ExternalCommandTarget,
        result: std::result::Result<(), String>,
    ) {
        let (success, failure) = match command {
            ExternalCommandKind::Browser => ("Opened in browser", "Couldn't open browser"),
            ExternalCommandKind::Clipboard => ("Copied URL", "Couldn't copy URL"),
        };
        match result {
            Ok(()) => {
                if let ExternalCommandTarget::TrackContext {
                    track_id,
                    generation,
                } = target
                    && self
                        .state
                        .track_context_menu
                        .as_ref()
                        .is_some_and(|menu| menu.context.track.id == track_id)
                    && self.state.track_context_generation == generation
                {
                    self.state.track_context_menu = None;
                }
                self.state.notify(success, false);
            }
            Err(error) => self.state.notify(&format!("{failure}: {error}"), true),
        }
    }
}

async fn run_external_command(
    command: ExternalCommandKind,
    url: &str,
    cancellation: &tokio_util::sync::CancellationToken,
) -> crate::error::Result<()> {
    match command {
        ExternalCommandKind::Browser => crate::app::browser::open_browser(url, cancellation).await,
        ExternalCommandKind::Clipboard => {
            crate::platform::clipboard::copy_url(url, cancellation).await
        }
    }
}
