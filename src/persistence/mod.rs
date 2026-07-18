//! JSON persistence: paths, atomic writes, and schema migrations.

pub mod json_store;
pub mod migrations;
pub mod paths;
pub mod resume;
pub mod session;
pub mod writer;

pub use paths::AppPaths;
