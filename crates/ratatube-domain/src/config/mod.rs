//! Configuration document model. Loading and validation are service work.

pub mod model;
pub mod theme;

pub use model::{Config, IconMode, ResumeMode};
pub use theme::{ThemeFamily, ThemeMode, ThemeName};
