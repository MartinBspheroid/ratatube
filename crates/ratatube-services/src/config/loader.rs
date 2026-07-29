//! Configuration loading and validation.

use std::path::Path;

use crate::config::model::{CONFIG_SCHEMA_VERSION, Config};
use crate::persistence::json_store;
use ratatube_domain::error::{AppError, Result};

const MAX_SEARCH_RESULTS: usize = 100;
const MAX_HISTORY_ENTRIES: usize = 10_000;

/// Load configuration from `path`.
///
/// A missing file yields the default configuration. A malformed file is an
/// error and must never be silently reset (PRD section 11.4).
pub fn load(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let config: Config = json_store::read(path)?;
    if config.schema_version > CONFIG_SCHEMA_VERSION {
        return Err(AppError::UnsupportedSchema {
            path: path.to_path_buf(),
            found: config.schema_version,
            supported: CONFIG_SCHEMA_VERSION,
        });
    }
    validate(&config)?;
    Ok(config)
}

/// Inspect configuration without creating backups or changing filesystem state.
pub fn inspect(path: &Path) -> Result<Option<Config>> {
    if !path.exists() {
        return Ok(None);
    }
    let config: Config = json_store::read_only(path)?;
    if config.schema_version > CONFIG_SCHEMA_VERSION {
        return Err(AppError::UnsupportedSchema {
            path: path.to_path_buf(),
            found: config.schema_version,
            supported: CONFIG_SCHEMA_VERSION,
        });
    }
    validate(&config)?;
    Ok(Some(config))
}

/// Persist configuration atomically.
pub fn save(path: &Path, config: &Config) -> Result<()> {
    json_store::atomic_write(path, config)
}

/// Validate configuration values, returning a readable message on failure.
fn validate(config: &Config) -> Result<()> {
    if config.playback.default_volume > 100 {
        return Err(AppError::Config(
            "playback.defaultVolume must be between 0 and 100".to_string(),
        ));
    }
    if config.search.result_limit == 0 {
        return Err(AppError::Config(
            "search.resultLimit must be at least 1".to_string(),
        ));
    }
    if config.search.result_limit > MAX_SEARCH_RESULTS {
        return Err(AppError::Config(format!(
            "search.resultLimit must not exceed {MAX_SEARCH_RESULTS}"
        )));
    }
    if config.history.max_entries > MAX_HISTORY_ENTRIES {
        return Err(AppError::Config(format!(
            "history.maxEntries must not exceed {MAX_HISTORY_ENTRIES}"
        )));
    }
    if config.ui.progress_refresh_ms < 100 {
        return Err(AppError::Config(
            "ui.progressRefreshMs must be at least 100".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unbounded_search_and_history_configuration() {
        let mut config = Config::default();
        config.search.result_limit = MAX_SEARCH_RESULTS + 1;
        assert!(validate(&config).is_err());

        let mut config = Config::default();
        config.history.max_entries = MAX_HISTORY_ENTRIES + 1;
        assert!(validate(&config).is_err());
    }
}
