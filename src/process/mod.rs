//! External process supervision.

pub mod supervisor;

pub use supervisor::{DependencyStatus, install_hint, probe, require};
