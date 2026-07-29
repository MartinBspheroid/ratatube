# Adversarial Remediation Verification

Verified on 2026-07-18 in `/Users/martinblasko/Code/playground/yt` with Rust 1.93.1. Tests use disposable roots or fake adapters except where explicitly labeled real/live.

## Release gate

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed. |
| `cargo check --locked --all-targets` | Passed. |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed with no warnings. |
| `cargo test --locked --all-targets` | Passed: 126 unit, 6 CLI, 18 UI, 1 real mpv, 11 deterministic yt-dlp; 3 live tests intentionally ignored by the default gate. |
| `cargo test --locked --test yt_dlp -- --ignored --nocapture` | Passed: live search, playlist fetch, and stream resolution, 3/3. |
| `cargo build --locked --release` | Passed. |
| `git diff --check` | Passed. |
| `cargo audit --version` | Not executed: Cargo reported that the `audit` subcommand is not installed. No advisory-scan claim is made. |

## Original-failure dispositions

- Delayed resolution responsiveness: a 30-second pending operation does not prevent an independent action arriving within 50 ms.
- Invalid queue document: bounds, duplicate, omission, cursor, and play-history fixtures are rejected; rendering uses checked access.
- Doctor/log mutation: six black-box CLI tests verify read-only absent/malformed/unwritable behavior, seeded-log preservation, continued dependency checks, and required CLI query syntax. An isolated release `doctor` run passed.
- Operation cancellation/stale results: registry/reducer tests cover same-kind supersession and playback/import/radio/details identity.
- Persistence truth: ordered-writer tests cover coalescing, errors, and flush; full command-path coverage reloads the durable queue.
- mpv correlation/recovery: fake IPC tests cover matching success/error, reordered IDs, timeout, malformed frame, EOF, and delayed acknowledgements; the real mpv integration passed.
- Schema versions and size symmetry: queue/history/playlist/session v0/v1/v99 policies and exact/over-limit writes are covered.
- Inert config: removed keys are rejected; `config.example.json` is parsed by a test.
- Help/search/notification/deletion UX: reducer, keymap, and 80x24 buffer tests cover restoration, scrolling, query provenance, elapsed expiry, bulk confirmation, and queue undo.
- Process/security boundaries: fake argv, exact host parsing, signed-URL redaction, stdout/stderr limits, and thumbnail dimensions are covered.
- Import/history integrity: reason-specific fixtures, exact video routing, media-position listening, completed-play/attempt metrics, local time, count, and date boundaries are covered.

## Explicit remaining conditions

- P25 command/runtime ownership remains concentrated. Characterization coverage and caching reduce risk, but extraction is still planned work.
- No license or repository remote was present. Cargo is non-publishable and docs block distribution until the owner chooses terms.
- Not verified here: Linux/Windows builds, physical mouse injection, screen-reader behavior, multi-instance writes, disk-full/power-loss recovery, real mpv crash/reconnect, authenticated or geo-restricted media, mutation testing, coverage percentage, memory profiling, and 1k/10k/100k latency budgets.
- Dependency advisory scanning remains required before external release because `cargo-audit` was unavailable.
