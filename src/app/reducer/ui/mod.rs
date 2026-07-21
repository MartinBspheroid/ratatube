//! UI-half reducers: transitions that touch only [`UiState`] (plus read-only
//! domain facts for clamping). The future client owns these; the daemon
//! never sees them.

pub(super) mod modals;
pub(super) mod navigation;
pub(super) mod presentation;
