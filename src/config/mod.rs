//! Configuration file model, loading, and validation.

pub mod loader;
pub mod model;

pub use loader::load;
pub use model::{Config, IconMode};
