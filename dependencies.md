# Dependency rationale

Runtime dependencies are intentionally listed here so removals and feature reductions have an owner-visible reason.

| Dependency | Why it exists |
|---|---|
| chrono | Persisted UTC timestamps, local display, and strict calendar parsing. |
| clap | Typed CLI and generated help. |
| crossterm | Terminal events, raw mode, mouse capture, and window dimensions. |
| directories | Platform-native config/data paths. |
| futures-util | Asynchronous terminal event stream utilities. |
| image | Bounded thumbnail decoding. |
| rand | Queue shuffle. |
| ratatui | Terminal layout and widgets. |
| ratatui-image | Kitty/iTerm/halfblock thumbnail protocols. |
| serde / serde_json | Versioned JSON configuration and persistence plus mpv IPC frames. |
| thiserror | Stable typed application errors. |
| tokio / tokio-util | Event loop, process/socket I/O, channels, timeouts, and cancellation tokens. |
| tracing / tracing-subscriber | Structured file diagnostics with configurable levels. |
| ulid | Stable locally generated playlist identifiers. |
| unicode-width | Terminal-cell width validation and width-safe component truncation. |
| which | Dependency discovery for `doctor` and startup checks. |
| tempfile (dev) | Isolated filesystem integration tests. |
