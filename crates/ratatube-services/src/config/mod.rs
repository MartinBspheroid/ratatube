//! Configuration loading and validation over the domain config model.

pub mod loader;

pub use loader::{inspect, load};
pub use ratatube_domain::config::{
    Config, IconMode, ResumeMode, ThemeFamily, ThemeMode, ThemeName, model, theme,
};
