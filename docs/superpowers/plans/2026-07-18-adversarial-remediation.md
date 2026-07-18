# Adversarial Review Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve every confirmed or actionable issue in `ADVERSARIAL_REVIEW.md`, verify disputed hypotheses safely, and leave the repository with production-oriented runtime, persistence, UX, content, documentation, and agent-readiness contracts.

**Architecture:** Fix externally visible failures before restructuring. Establish validated persistence and supervised operation boundaries first; move playback/process work behind those boundaries; then repair information and interaction contracts. Refactor `App` only after characterization tests make behavior safe to move.

**Tech Stack:** Rust 2024, Tokio, Ratatui/Crossterm, mpv JSON IPC, yt-dlp subprocesses, Serde JSON, Cargo tests/Clippy/rustfmt.

## Global Constraints

- Preserve the 12 pre-existing modified source/test files and their intent; never revert unrelated hunks.
- Use TDD for every behavior change: add one failing test, run it and confirm the expected failure, implement minimally, then rerun focused and broader gates.
- Use the declared Cargo toolchain and locked dependencies.
- Do not run `doctor` against the real application directory. CLI tests must inject an explicit disposable data root before execution.
- Do not implement a YouTube-only allowlist until supported-source policy is documented; add argv termination and structural URL validation first.
- Keep one ordered persistence writer; do not spawn unordered save tasks.
- Do not await mpv acknowledgements or yt-dlp resolution inside the central UI event loop.
- Do not commit or push while unrelated dirty changes are present. End each iteration with a focused diff review and recorded verification instead.
- No production TODOs, silent catches, debug payload logging, or undocumented config fields.

## Issue Coverage

| Review issues | Remediation iteration |
|---|---|
| P2 queue panic, P7 migration, P23 read/write limit | 1 |
| P1 resolver freeze, P4 task lifecycle, P15 details loading | 2 |
| P6 mpv acknowledgements and disconnect recovery | 3 |
| P3 log truncation, P5 persistence truth, P14 doctor | 4 |
| P11 output/image limits, P12 argv boundary, P13 log redaction | 5 |
| P8 inert config, P16 import metrics, P17 history duration, P18 exact URLs, P19 Top History | 6 |
| P9 Help, P10 mouse, P20 search, P21 notification timing, P22 destructive safety | 7 |
| P24 docs/CI/AI readiness and package metadata | 8 |
| P25 split ownership; long-term dependency/rendering/deletion opportunities | 9 |
| All confirmed failures, hypotheses, scores, and stale review claims | 10 |

---

### Iteration 1: Validated Persistence Boundary

**Files:**
- Modify: `src/queue/model.rs`, `src/queue/service.rs`
- Modify: `src/persistence/json_store.rs`, `src/persistence/migrations.rs`
- Modify: `src/history/service.rs`, `src/playlists/service.rs`, `src/persistence/session.rs`
- Test: colocated unit tests in those modules

**Interfaces:**
- Produces: `Queue::validate() -> Result<()>`; version-aware document loading; symmetric read/write byte limits.
- Consumes: existing `AppError::MalformedData` and atomic-write behavior.

- [x] Add failing queue tests for out-of-range order, duplicate order, omitted tracks, invalid position, and valid shuffled order.
- [x] Run focused queue tests; invalid documents were accepted before validation was implemented.
- [x] Implement `Queue::validate` and invoke it from `queue::service::load`; preserve malformed source through existing backup behavior and return a structured error.
- [x] Add checked access in queue render/playback boundaries as defense in depth.
- [x] Add failing service tests for version 0, version 1, and version 99 queue/history/playlist/session documents.
- [x] Replace the generic no-op migration with explicit v0 transformation and future-version rejection at every document loader.
- [x] Add a failing test proving `atomic_write` refuses serialized output above the same 16 MiB budget used by reads while preserving the previous file.
- [x] Implement a shared `MAX_DOCUMENT_BYTES` pre-write guard and actionable error.
- [x] Run focused suites, then `cargo test --locked --all-targets` and `cargo clippy --locked --all-targets -- -D warnings`.
- [x] Review the focused diff and mark P2/P7/P23 status in `ADVERSARIAL_REVIEW.md`.

**Verification (2026-07-18):** `cargo fmt --all && cargo test --locked --all-targets && cargo clippy --locked --all-targets -- -D warnings && git diff --check` passed: 83 unit tests, 1 real-mpv integration test, 16 UI tests, and 7 deterministic yt-dlp tests passed; 3 network tests remained explicitly ignored.

**Acceptance criteria:** The reproduced `[99]` queue cannot enter runtime state; every persisted document has an explicit version policy; the app cannot write a file it refuses to read.

### Iteration 2: Supervised Playback and Background Operations

**Files:**
- Create: `src/app/operations.rs`
- Modify: `src/app/mod.rs`, `src/app/action.rs`, `src/app/reducer.rs`, `src/app/state.rs`
- Modify: `src/media/yt_dlp.rs`
- Test: new operation tests plus reducer tests

**Interfaces:**
- Produces: generation-tagged `PlaybackResolveStarted/Completed/Failed/Cancelled`, import/radio/details operation IDs, one task owner with cancellation.
- Consumes: `YtDlp`, action channel, current queue track identity.

- [x] Add a Tokio test with a 30-second delayed operation proving an independent Quit action is processed within 50 ms.
- [x] Add reducer/operation tests proving stale playback, import, radio, and details completions are ignored.
- [x] Add a test proving details failure transitions from Loading to Failed.
- [x] Implement `OperationId` and an `OperationRegistry` owning cancellation tokens/join handles for playback, import, radio, details, thumbnails, search, prefetch, mix, and session resume.
- [x] Replace awaited `resolve_and_play` with one serialized playback request task carrying operation ID, queue position, and track ID.
- [x] Move retry/continue-on-error policy into the supervised playback pipeline without mutating current track until resolution succeeds.
- [x] Make Esc cancel Fetching import and invalidate late completion; disabling radio cancels and invalidates its refill.
- [x] Add explicit truthful Loading/Failed states for details, import, and playback; user cancellation closes the cancelled modal while the registry records cancellation.
- [x] Ensure shutdown cancels operations, terminates kill-on-drop children, and waits for bounded task cleanup.
- [x] Run focused operation/reducer tests, then full tests and Clippy.
- [x] Review focused diff and mark P1/P4/P15 resolved with delayed regression evidence.
- [x] Fix the user-reported repeated-volume regression: controller events are synchronized and successful volume commands update the local snapshot before the next key press.

**Verification (2026-07-18):** `cargo fmt --all && cargo test --locked --all-targets && cargo clippy --locked --all-targets -- -D warnings && git diff --check` passed: 92 unit tests, 1 real-mpv integration test, 16 UI tests, and 7 deterministic yt-dlp tests passed. The operation test keeps a resolver pending for 30 seconds while an independent action arrives within a 50 ms deadline. The volume regression test proves two immediate increments accumulate from 50% to 54%.

**Acceptance criteria:** External latency cannot block the event loop; cancelled/superseded work cannot mutate current state; every loading state terminates truthfully.

### Iteration 3: Acknowledged and Recoverable mpv Boundary

**Files:**
- Modify: `src/playback/ipc.rs`, `src/playback/controller.rs`, `src/playback/mpv.rs`, `src/playback/events.rs`
- Modify: `src/app/mod.rs`, `src/app/action.rs`
- Test: `tests/mpv_ipc.rs` plus a deterministic fake Unix-socket server test

**Interfaces:**
- Produces: per-request `Result<Value>` with timeout; explicit fire-and-forget API where acknowledgement is unnecessary; disconnect/restart actions.

- [x] Add fake-server tests for successful response, error response, reordered IDs, timeout, malformed frame, and EOF.
- [x] Extend `MpvFrame` with `request_id`; implement one reader owner and a pending-request map of oneshot senders.
- [x] Make load/seek/property writes await bounded acknowledgements in one owned command worker outside the UI loop; retain the unsolicited event stream.
- [x] Add a failing test showing `wait_for_socket` reports early child exit rather than waiting the full timeout.
- [x] Detect child exit with `try_wait`; include stderr/exit context where available.
- [x] On IPC shutdown, invalidate stale handles and run three bounded background restart/reconnect attempts with visible result state.
- [x] Keep the real mpv integration test's explicit prerequisite skip message; in this environment mpv/ffmpeg were present and the test executed successfully.
- [x] Run fake IPC tests, real mpv test, full tests, and Clippy.

**Verification (2026-07-18):** The fake IPC suite covers correlated success/error, reordered IDs, 20 ms timeout, malformed frames, and EOF. A control call returns within 50 ms while the fake acknowledgement is delayed 200 ms. `/usr/bin/false` early-exit detection returns in ~25 ms against a 2-second timeout. The complete gate passed with 100 unit tests, 1 real-mpv test, 16 UI tests, 7 deterministic yt-dlp tests, rustfmt, Clippy, and `git diff --check`.

**Acceptance criteria:** Command success means mpv accepted the command where required; disconnection is detected and recoverable; no acknowledgement wait blocks UI dispatch.

### Iteration 4: Truthful Diagnostics, Logging, and Persistence Outcomes

**Files:**
- Create: `src/diagnostics.rs`, `src/persistence/writer.rs`
- Modify: `src/main.rs`, `src/process/supervisor.rs`, `src/persistence/paths.rs`, `src/app/mod.rs`, `src/app/action.rs`
- Test: new CLI integration tests and persistence-writer unit tests

**Interfaces:**
- Produces: `--data-dir`; read-only doctor report; optional explicit repair; ordered coalescing writer with acknowledged flush.

- [x] Add CLI tests using a temporary `--data-dir` for absent config, malformed config, missing dependency, unwritable path, and seeded non-empty log.
- [x] Add `--data-dir` to CLI and route `doctor` before directory creation and TUI log initialization.
- [x] Report absent config as defaults-in-use; continue diagnostics after malformed config and state that doctor made no changes.
- [x] Check curl as optional artwork capability and inspect data-dir writability without a write probe.
- [x] Replace truncate logging with restrictive append plus bounded one-generation rotation; warn visibly if logging cannot initialize.
- [x] Add writer tests for ordering, coalescing, save error, flush on shutdown, and no stale write resurrection.
- [x] Route queue/history/session persistence through one ordered writer with typed outcomes. Playlist CRUD intentionally remains synchronous in the serial action owner because its existing success/failure result is immediately acknowledged; moving it asynchronously would require rollback or delayed state mutation to preserve the same truth contract.
- [x] Surface an explicit `Changes are not durable` error when an optimistic queue/history/session write fails.
- [x] Run CLI/writer tests in temporary roots, full tests, Clippy, and rustfmt.

**Verification (2026-07-18):** Five black-box doctor tests prove no absent root/log creation, seeded-log preservation, malformed-config preservation without `.bak`, continued dependency reporting, and read-only unwritable-path detection. Logging append, rotation, and Unix 0600 tests pass. Writer tests prove serial ordering, same-key coalescing, failure outcomes, and flush durability. The complete gate passed with 105 unit tests, 5 CLI tests, 1 real-mpv test, 16 UI tests, and 7 deterministic yt-dlp tests.

**Acceptance criteria:** Diagnostics do not erase or manufacture health; persistence failures are visible and ordered; all black-box tests are isolated from real user data.

### Iteration 5: External Process and Resource Boundaries

**Files:**
- Modify: `src/media/import.rs`, `src/media/yt_dlp.rs`, `src/app/operations.rs`, thumbnail handling
- Modify: `src/config/model.rs`
- Test: `tests/yt_dlp.rs` and resource-limit tests

**Interfaces:**
- Produces: parsed supported input type; `--` positional separator; bounded stdout/stderr/entry/image budgets; structured TooLarge errors.

- [x] Add a fake-binary argv test for search, exact video, playlist, details, and stream resolution; mix reuses the verified playlist path.
- [x] Add `--` immediately before positional URL/pseudo-URL arguments and prove query text remains one `ytsearchN:` argument.
- [x] Replace substring URL detection with structural scheme/authority/host/path/query parsing while preserving documented YouTube variants and rejecting lookalike hosts.
- [x] Add bounded-output tests at the exact byte limit and one byte over for stdout and stderr.
- [x] Stream both pipes concurrently, kill/reap on budget exceedance or timeout, and return `ResourceLimit` rather than partial JSON.
- [x] Add bounded search/history configuration with safe maxima of 100 and 10,000.
- [x] Bound thumbnail transfer to 5 MiB and configure 4096x4096/64 MiB decoder limits before decode; test oversized dimensions.
- [x] Redact URLs from process stderr and log only action/event discriminants, never payload URLs or binary bytes; regression-test signed-URL redaction.
- [x] Run yt-dlp tests, targeted resource tests, full tests, and Clippy.

**Verification (2026-07-18):** Fake argv capture verifies every direct positional path. Subprocess tests accept exact stdout/stderr limits and reject one byte over. URL tests reject credential tricks, lookalike suffixes, and non-HTTP schemes. Thumbnail dimension and signed-URL redaction tests pass. The complete gate passed with 110 unit tests, 5 CLI tests, 1 real-mpv test, 16 UI tests, and 10 deterministic yt-dlp tests.

**Acceptance criteria:** Subprocess inputs have a defined positional and URL contract; external data cannot grow memory without enforced bounds; logs redact sensitive payloads.

### Iteration 6: Information Integrity and Configuration

**Files:**
- Modify: `src/config/model.rs`, `src/config/loader.rs`
- Modify: `src/media/yt_dlp.rs`, `src/playlists/import.rs`, `src/media/mod.rs`
- Modify: `src/history/model.rs`, `src/history/service.rs`, playback-history integration
- Modify: `src/app/action.rs`, `src/app/reducer.rs`, `src/app/mod.rs`, `src/ui/views.rs`
- Test: existing module tests plus new fixtures

**Interfaces:**
- Produces: typed `SkipReason`; exact-video fetch action; media-position listening accumulator; truthful Top History model.

- [x] Remove inert `audioOnly`, `resolveBeforePlayback`, and `showFooterHints`; strict config deserialization now rejects those keys instead of accepting no-op settings.
- [x] Add a mixed playlist fixture with deleted/private/unavailable/missing-id/missing-title entries; duplicate attribution is independently tested in the import builder.
- [x] Change normalization from `Option<Track>` to `SkipReason` plus traceable per-reason counts through the import review UI.
- [x] Add an exact-video reducer test proving pasted/CLI URLs emit `RunExactVideo`, never `RunSearch`; runtime uses `fetch_video`.
- [x] Add controlled listened-time tests covering playing deltas, pause, explicit seek reset, backwards/restart jumps, and large external seeks. Outcome handling remains covered by reducer/history tests.
- [x] Replace wall-clock “listened” with bounded positive media-position deltas while playing; speed semantics intentionally count media seconds, not wall seconds.
- [x] Define Top History completed plays vs attempts, distinct-row count, mode-specific delete hints, and explicitly local timestamps.
- [x] Add compact-count/date boundaries and reject invalid upload dates rather than formatting by length.
- [x] Run information-integrity tests, full tests, and Clippy.

**Verification (2026-07-18):** Tests cover removed inert config keys, exact-video effect routing, each import rejection reason, deduplication, listened-time pause/seek/jump behavior, completed-play/attempt aggregation, local-mode UI wording, compact-count boundaries, and leap-day/calendar validation. The complete gate passed with 116 unit tests, 5 CLI tests, 1 real-mpv test, 16 UI tests, and 11 deterministic yt-dlp tests.

**Acceptance criteria:** Every displayed category, duration, count, date, URL identity, and config option is traceable to data capable of supporting the claim.

### Iteration 7: Interaction, Help, and Recovery UX

**Files:**
- Create: `src/input/commands.rs`, `src/ui/hit_test.rs`
- Modify: `src/input/keymap.rs`, `src/ui/views.rs`, `src/ui/widgets.rs`, `src/ui/layout.rs`
- Modify: `src/app/state.rs`, `src/app/reducer.rs`, `src/app/mod.rs`
- Test: `tests/ui_snapshots.rs` plus key/mouse tests

**Interfaces:**
- Produces: canonical command metadata for keymap/help/footer; renderer-derived hit zones; timestamped notifications; consistent destructive-action policy.

- [ ] Add failing parity tests ensuring every displayed command is valid in its view/mode and every key command is reachable in Help.
- [x] Render Help from canonical command metadata with scrolling and previous-view restoration on Esc/`?`.
- [ ] Add semantic viewport tests at 80x24, 100x30, and 120x40 proving the final command is reachable.
- [ ] Add synthetic mouse tests for Search/filter/split panes/scroll offsets and same-target double-click.
- [x] Publish hit zones from the renderer rather than duplicating row arithmetic in mouse handling.
- [x] Preserve editable and submitted search queries separately; show `Results for ...` and support retry/refine after failure.
- [x] Store notification creation/expiry timestamps with severity-specific durations.
- [x] Confirm bulk clears; provide bounded undo for frequent single-item deletion; make footer hints mode-specific.
- [x] Fix Home `N` behavior/copy, empty prompt validation, dependency recovery guidance, refresh/sync wording, and message-log timestamps/source.
- [x] Run UI/key tests, full tests, fmt, and Clippy. Physical mouse injection remains unavailable.

**Verification (2026-07-18):** Reducer/keymap tests cover Help restoration, scrolling, deletion confirmation/undo, notification deadlines, and formerly dead Home `N`. UI tests prove final Help commands are reachable at 80x24 and Search publishes renderer-derived hit rows. Full locked gates pass.

**Acceptance criteria:** Every instruction maps to a working action; mouse target equals rendered target; loading/error/destructive states are discoverable and recoverable.

### Iteration 8: Documentation, CI, Packaging, and Agent Readiness

**Files:**
- Create: `README.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `config.example.json`, `LICENSE`
- Create: `.github/workflows/ci.yml`, `rust-toolchain.toml`
- Modify: `Cargo.toml`, `.gitignore`, source PRD references

- [x] Write README covering purpose/scope, prerequisites, install, first playback, doctor, config/data/log paths, controls, terminals, limitations, and recovery.
- [x] Write architecture boundaries and invariants for command processing, playback ownership, task cancellation, persistence, and generated/runtime files.
- [x] Add executable config example generated/validated by a test.
- [x] Add contribution gate commands and explain deterministic, real-adapter, and ignored network tests.
- [x] Add CI for fmt, check, Clippy, deterministic tests, and explicit integration-skip reporting.
- [ ] Fill Cargo description/license/repository/readme/rust-version metadata; choose license with user confirmation if none is already intended.
- [x] Replace unavailable PRD references with a checked-in `PRD.md` and architecture contract.
- [x] Document runtime artifacts including `ruvector.db` and move ignore rules into repository-local `.gitignore`.
- [x] Execute documented build, test, CLI Help, and read-only doctor commands against an isolated data root.

**Verification (2026-07-18):** The checked-in config parses in a unit test; CLI Help marks `<QUERY>...` required and a black-box test enforces exit 2 when absent. Release build and isolated doctor pass. Cargo is `publish = false`; no license or remote was invented, and distribution is explicitly blocked pending owner choice.

**Acceptance criteria:** A human or AI agent can install, run, test, modify, and recover the project without source archaeology or hidden machine state.

### Iteration 9: Architectural Consolidation and Performance

**Files:**
- Create: `src/app/runtime.rs`, `src/app/command.rs`, `src/playback/session.rs`, `src/ui/model.rs`
- Modify/split: `src/app/mod.rs`, `src/app/reducer.rs`, `src/app/state.rs`, `src/ui/views.rs`
- Modify: Cargo dependency features
- Test: characterization workflows and performance harness

- [x] Add a full command-path characterization test covering reduce, effect execution, service dispatch, and durable queue reload.
- [ ] Make one command processor own transition plus effects; remove reducer/service dual interpretation.
- [ ] Move playback resolution/prefetch/mpv/task lifecycle into `PlaybackSession`.
- [ ] Separate domain/application state from Ratatui widget state, image protocols, and hit geometry.
- [ ] Build immutable view models so UI no longer receives concrete persistence services.
- [x] Cache unchanged filtered indices and Top History aggregation with mutation invalidation. Visible-window virtualization remains open.
- [ ] Coalesce high-frequency playback redraws while rendering input/resize/error transitions immediately.
- [ ] Add 1k/10k/100k record latency/draw-count benchmarks and define budgets.
- [x] Remove the unused direct `anyhow` dependency and document every remaining direct dependency. Public-surface reduction remains open.
- [x] Narrow Tokio from `full` to the verified runtime/process/io/net/sync/time feature set. Image features remain unchanged because `ratatui-image` unifies its decoder requirements.
- [ ] Keep each resulting file under the repository's 250-line rule where practical, without creating shallow pass-through modules.

**Acceptance criteria:** One local place explains each command; domain tests do not require Ratatui or real processes; large data and event rates have measured budgets.

**Status:** P25 is mitigated, not closed. Mechanical file splitting, immutable view models, visible-window virtualization, coalesced redraw metrics, and 1k/10k/100k budgets remain planned structural work. `ARCHITECTURE.md` makes that debt explicit so it cannot be mistaken for completion.

### Iteration 10: Final Adversarial Verification and Review Closure

**Files:**
- Modify: `ADVERSARIAL_REVIEW.md`
- Create: `docs/remediation-verification.md`

- [x] Re-run original confirmed failures through isolated regression tests and an explicit temporary-root doctor invocation.
- [ ] Execute the disputed argv, mouse, output-memory, multi-instance, mpv crash, disk-failure, and schema tests.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run `cargo check --locked --all-targets`.
- [x] Run `cargo clippy --locked --all-targets -- -D warnings`.
- [x] Run `cargo test --locked --all-targets` and explicitly run/record live tests when network is available.
- [x] Review full diff, check generated files, dependency rationale, docs commands, and ignored files.
- [x] Update each P1–P25 entry with Resolved, Mitigated, or an exact remaining condition.
- [x] Re-score production readiness from evidence rather than preserving the original score.
- [x] Capture a Learning Record for repeated test-command and domain-assertion mistakes.

**Verification (2026-07-18):** See `docs/remediation-verification.md` for exact outcomes and unexecuted checks. The disputed multi-instance, physical mouse, real crash, disk-full, and large-scale benchmark scenarios remain explicitly unverified rather than being presented as passed.

**Acceptance criteria:** No issue is silently dropped; every original finding has fresh evidence and an explicit final disposition.
