//! Ordered queue, history, and session persistence submission.

use crate::app::App;
use crate::error::Result;

impl App {
    pub(super) fn submit_persistence(
        &mut self,
        key: &str,
        description: &str,
        job: impl FnOnce() -> Result<()> + Send + 'static,
    ) {
        let Some(writer) = &self.persistence_writer else {
            if let Err(err) = job() {
                self.state.notify(
                    &format!("Could not save {description}: {err}. Changes are not durable."),
                    true,
                );
            }
            return;
        };
        if let Err(err) = writer.submit(key, description, job) {
            self.state
                .notify(&format!("Could not queue {description} save: {err}"), true);
        }
    }

    pub(super) fn persist_queue(&mut self) {
        let path = self.paths.queue_file();
        let queue = self.state.queue.clone();
        self.submit_persistence("queue", "queue", move || {
            crate::queue::service::save(&path, &queue)
        });
    }

    pub(super) fn persist_history(&mut self) {
        if let Some(history) = self.history.clone() {
            self.submit_persistence("history", "history", move || history.save());
        }
    }
}
