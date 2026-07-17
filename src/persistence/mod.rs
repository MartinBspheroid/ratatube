//! JSON persistence: paths, atomic writes, and schema migrations.

pub mod json_store;
pub mod migrations;
pub mod paths;

pub use paths::AppPaths;
