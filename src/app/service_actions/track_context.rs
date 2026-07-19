//! Service-backed opening of the universal track context menu.

use crate::app::App;

impl App {
    /// Resolve the active track with optional history data and open its menu.
    pub(super) fn open_track_context(&mut self) {
        crate::app::track_context::open_track_context(&mut self.state, self.history.as_ref());
    }
}
