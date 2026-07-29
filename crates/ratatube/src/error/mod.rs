//! Crate-wide error types, defined in the domain crate so services and the
//! pure core classify failures the same way.

pub use ratatube_domain::error::{AppError, ErrorCategory, Result};
