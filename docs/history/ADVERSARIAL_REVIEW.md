# Six-Agent Adversarial Project Review

Review target: current working tree of `ytm-tui` at `d532be7` on 2026-07-18.

The checkout contained 12 pre-existing modified files. Reviewers inspected that working tree, not only `HEAD`. No application source was changed by this review. One report file—the document you are reading—was added as the deliverable.

## Post-remediation addendum — 2026-07-18

The review body below preserves the original evidence and conclusions. The inline remediation status under each P1–P25 finding is the current authority.

P1–P24 have been corrected or, for licensing, converted into an explicit owner decision that blocks publication instead of an invented legal grant. P25 is materially reduced but remains open structural debt: full command-path coverage now exists, expensive repeated derivations are cached, and architecture ownership is documented, but the central runtime has not been mechanically split without sufficient characterization.

Current verification passed `cargo fmt --all -- --check`, locked all-target check, strict Clippy, locked all-target tests, release build, `git diff --check`, a real mpv test, and all three explicitly invoked live yt-dlp tests. Totals: 126 unit, 6 CLI, 18 UI, 1 real mpv, 11 deterministic yt-dlp, and 3 live yt-dlp tests. `cargo-audit` was not installed, so no dependency advisory scan is claimed. Cross-platform builds, physical mouse injection, screen-reader tooling, disk-full/power-loss, and multi-instance conflict tests remain unverified.

**Current production readiness: Ready with minor conditions for controlled local use.** External distribution still requires an owner-selected license and a dependency advisory scan. P25 consolidation should precede major feature expansion.

## Review method and evidence limits

Six isolated reviewers first inspected the repository independently: architecture, destructive QA, product/UX, performance/security/reliability, information integrity, and content quality. They then received an attributed digest of the other five reports and cross-examined at least three claims each.

The initial performance/security reviewer and initial QA reviewer were blocked by an automated safety classifier and were replaced. A later performance reviewer failed to return and was also replaced. The final six reports are from six completed independent agents.

During review, several agents assumed XDG variables would isolate `directories::ProjectDirs` on macOS. They did not. Running `doctor` reached the real Application Support directory and `File::create` truncated `ytm-tui.log`. This is evidence of a product defect and a review-procedure failure. The prior log contents are not recoverable through this application. No queue, history, playlist, session, or config document was changed. A reviewer-launched TUI process was found after the reviews and terminated by exact PID.

### Checks executed

| Command or scenario | Result | Reproducibility / limitation |
|---|---|---|
| `cargo fmt --all -- --check` | Passed | Repeated by multiple reviewers. |
| `cargo check --locked --all-targets` | Passed | Also passed with all features in an isolated target directory. |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed | No warnings. |
| `cargo test --locked --all-targets` | Passed: 90 tests; 3 live tests ignored | 66 library, 1 mpv IPC, 16 UI, 7 mocked yt-dlp. |
| `cargo test --locked --test yt_dlp -- --ignored --nocapture` | Passed: 3/3 | Live search, playlist fetch, and stream resolution passed on this machine. |
| `cargo test --locked --test ui_snapshots visual_dump_help_and_modal -- --nocapture` | Test passed; output exposed a defect | At 100x34 Help ended at `N New playlist`; later commands were unreachable. |
| `cargo metadata --no-deps --format-version 1` | Passed | Confirmed absent readme/license/repository/rust-version/package description. |
| `cargo tree --locked --duplicates` | Passed | Found broad image/Tokio features and duplicate transitive versions; no runtime failure inferred from duplicates alone. |
| `cargo run --locked -- --help` | Passed | Minimal CLI contract only. |
| `cargo run --locked -- play --help` then `cargo run --locked -- play` | Help advertised `[QUERY]...`; execution rejected omission | Reproduced CLI contract mismatch. |
| `cargo run --locked -- doctor` | Reported success on configured machine | Mutated real log and checked directories after creating them. Do not rerun for confirmation without an explicit isolated data root. |
| Clean-home `doctor` scenario | Printed `OK config` for a nonexistent file | macOS ProjectDirs behavior limited isolation attempts. |
| Missing executable doctor scenario with empty `PATH` | Correctly returned failure for mpv and yt-dlp | Confirms doctor is not universally false; its path/config/log semantics are the defect. |
| Malformed config through `doctor` | Exit 1; backup created; debug-shaped error only | Reproduced by QA. Exact shell harness was not retained in the agent transcript. |
| Persisted queue with one track and `order: [99]` | Panic at `src/ui/views.rs:515` after opening Queue | Reproduced by QA. Exact shell harness was not retained; steps are deterministic. |
| Delayed local yt-dlp mock during playback resolution | `q` remained unprocessed while resolution awaited | Reproduced by QA. Exact harness was not retained; code path independently confirms it. |
| `git diff --check` | Passed | No whitespace errors in the pre-existing changes. |

Not executed or unavailable: dependency advisory scan (`cargo-audit` not installed), mutation testing, coverage tooling, Windows/Linux builds, physical mouse injection, screen-reader tooling, large-dataset benchmark, full memory profiling, real mpv crash/reconnect, disk-full/power-loss simulation, and authenticated/geo-restricted YouTube scenarios.

## 1. Executive Summary

This is a functioning prototype with unusually good compiler hygiene for its age. It builds cleanly, passes strict Clippy, passes 90 default tests, passes a real mpv IPC smoke test, and passed three live yt-dlp success paths. The persistence writer attempts atomic replacement, modules have meaningful names, and the happy-path feature set is substantial.

It is not production-ready.

Two core failures were reproduced:

1. A syntactically valid persisted queue can crash normal rendering because cross-field invariants are never validated.
2. Stream resolution is awaited inside the only event loop, freezing input, rendering, timers, and playback-event processing while an external command is slow.

The most dangerous assumption is that an action/reducer vocabulary makes the application event-driven. In reality, one 1,589-line `App` still owns terminal input, UI hit-testing, process lifecycle, persistence, external calls, task spawning, playback coordination, and rendering. Actions are interpreted by the reducer and then again by a service dispatcher. External work has inconsistent identity and cancellation semantics.

The second dangerous assumption is that declared contracts are enforced. They are not: queue schema/invariants are trusted, migrations are disconnected, three config fields are inert, mpv request IDs are not correlated, import reason metrics are lossy, listening duration is mislabeled wall time, Help is incomplete, and `doctor` changes and erases state before diagnosing it.

Largest unknowns are large-dataset behavior, external-output memory bounds, multi-instance races, cross-platform behavior, real mouse interaction, and fault recovery under disk/process/network failure.

**Production readiness: Not ready without significant corrections.**

## 2. Top 25 Issues

### P1. Stream resolution freezes the sole event loop

> **Remediation status (2026-07-18): Resolved.** Stream resolution and its retry now run in one registry-owned, cancellable operation rather than being awaited by the action consumer. Completion carries operation identity, queue position, and track identity; stale completions are rejected. A Tokio regression keeps resolution pending for 30 seconds while an independent action is received within 50 ms.

- **Severity / category:** High — core workflow, reliability, responsiveness
- **Affected:** `App::run`, `execute`, `resolve_and_play` in `src/app/mod.rs:159`, `:703`, `:1427`; timeout in `src/media/yt_dlp.rs:15`
- **Evidence / reproduction:** QA used a delayed local yt-dlp mock. Search completed, stream resolution slept, and `q` was not processed. Code shows two sequential 60-second attempts and potentially further queue advancement.
- **Actual / expected:** Input, render, sleep timer, and mpv events stop while resolving. Resolution should be cancellable background work while the UI remains live.
- **Why / root cause:** A long-running effect is awaited by the sole action consumer.
- **Correction / effort:** Introduce one playback-request owner with generation IDs, cancellation, and completion/failure actions. Medium-large, 2–4 days plus tests.
- **Long-term impact:** Without correction, every new awaited external effect can freeze the whole application.
- **Confidence / status:** Very high — **Confirmed by execution**.

### P2. Persisted queue invariants are unvalidated and can panic

> **Remediation status (2026-07-18): Resolved.** `Queue::validate` now rejects invalid bounds, duplicates, incomplete order coverage, invalid positions, and invalid history indices during load; malformed input is preserved as a backup. Queue removal preserves the invariant, and rendering uses checked access. Regression coverage includes the reproduced `[99]` shape and valid shuffled queues. Full tests and Clippy pass.

- **Severity / category:** High — availability, persistence integrity
- **Affected:** `src/queue/service.rs:30`, `src/queue/model.rs`, `src/ui/views.rs:511`
- **Evidence / reproduction:** A one-track queue with `order: [99]` loaded as valid JSON; opening Queue panicked at `tracks[i]` on `ui/views.rs:515`.
- **Actual / expected:** Normal navigation crashes. Load should validate bounds, uniqueness, coverage, and position; bad data should be backed up and safely rejected or repaired.
- **Why / root cause:** Serde shape validity is treated as domain validity; render and playback use unchecked indices.
- **Correction / effort:** Add `Queue::validate`, invoke at load, use checked access defensively, preserve invalid input, and show recovery guidance. Small-medium, 1–2 days.
- **Long-term impact:** Prevents persistent crash loops and future-schema corruption from escaping into runtime.
- **Confidence / status:** Very high — **Confirmed by execution**.

### P3. Startup and `doctor` truncate diagnostic history

> **Remediation status (2026-07-18): Resolved.** `doctor` runs before directory/log initialization and is read-only under an explicit `--data-dir`. TUI logging uses restrictive append mode and rotates at 5 MiB. Tests preserve a seeded log byte-for-byte through doctor, prove append rather than truncate, verify rotation, and verify Unix 0600 permissions.

- **Severity / category:** Medium — operability, information loss
- **Affected:** `src/main.rs:29`, `src/main.rs:221`
- **Evidence / reproduction:** `init_tracing` uses `File::create` before command dispatch. Review runs left the real log at zero bytes. The exact amount of prior meaningful content is unknown.
- **Actual / expected:** Every launch replaces the prior log; running the diagnostic command after a crash destroys the primary diagnostic record. Logs should append with bounded rotation and sensitive-value redaction.
- **Why / root cause:** Logging lifecycle is shared by normal startup and diagnostics with truncate semantics.
- **Correction / effort:** Dispatch non-TUI commands before TUI logging, append with retention/rotation, and expose logging degradation. Under 1 day.
- **Long-term impact:** Restores incident evidence and makes `doctor` safe to recommend.
- **Confidence / status:** Very high — **Confirmed by code inspection and observed execution**. Materiality of lost historical content is unknown.

### P4. Background operations lack a coherent cancellation and supersession model

> **Remediation status (2026-07-18): Resolved.** `OperationRegistry` now owns cancellation tokens and join handles for playback, import, radio, details, thumbnail, search, prefetch, mix, and session work. Same-kind work is superseded, Esc cancels in-flight import, radio disable cancels refill, Quit performs bounded teardown, and reducer tests reject stale playback/import/radio/details results.

- **Severity / category:** Medium — concurrency, reliability
- **Affected:** detached tasks in `src/app/mod.rs`; import in `:1365`, radio around `:1314`, details/thumbnail at `:1512`
- **Evidence / reproduction:** `CancelImport` clears UI state but the task has no generation/token; late completion can reopen review. `RadioTracksLoaded` can append after radio is disabled. Several join handles are discarded.
- **Actual / expected:** Cancelled/superseded work can still mutate current state and consume resources. Every operation needs identity, cancellation, stale-result rejection, and shutdown ownership.
- **Why / root cause:** Task spawning was added feature by feature without a task supervisor.
- **Correction / effort:** Add an operation registry/task owner, cancellation tokens, bounded concurrency, and generation-tagged results. Medium, 3–6 days.
- **Long-term impact:** Prevents race bugs and process storms as features grow.
- **Confidence / status:** High — **Confirmed by code inspection**; individual race timing remains unexecuted.

### P5. Persistence failures can be presented as success

> **Remediation status (2026-07-18): Resolved for mutable runtime documents.** Queue, history, and session saves use one ordered coalescing writer with typed outcomes and shutdown flush; failures produce a visible `Changes are not durable` error. Playlist CRUD remains synchronous in the serial action owner and only reports success after its filesystem operation succeeds. Writer tests cover ordering, coalescing, failure, and flush durability.

- **Severity / category:** Medium — data integrity, status truthfulness
- **Affected:** queue/history/session saves in `src/app/mod.rs:759`, `:916`, `:1082`, `:1410`
- **Evidence / reproduction:** Several save errors are only logged. History can mutate in memory and announce success even if save fails.
- **Actual / expected:** Restart can resurrect supposedly deleted data or discard accepted changes. UI should receive typed persistence outcomes and either roll back or visibly mark state unsaved.
- **Why / root cause:** Persistence is an unacknowledged side effect after optimistic mutation.
- **Correction / effort:** Central ordered writer plus success/failure actions and acknowledged shutdown flush. Medium, 2–4 days.
- **Long-term impact:** Prevents silent divergence between memory and disk.
- **Confidence / status:** High — **Confirmed by code inspection**; fault injection still required.

### P6. mpv request IDs are not correlated with responses

> **Remediation status (2026-07-18): Resolved.** `MpvIpc` routes each response by `request_id` through a pending oneshot map and returns matching data, explicit mpv errors, protocol errors, disconnects, or a bounded timeout. Runtime controls use one owned command worker so acknowledgement latency cannot block terminal input. Fake-server tests cover reordered responses, malformed frames, EOF, and delayed acknowledgements; the real mpv integration test also passes. IPC shutdown starts three bounded background reconnect attempts.

- **Severity / category:** Medium — playback reliability, API semantics
- **Affected:** `src/playback/ipc.rs:20`, `:58`, `:118`; controller commands
- **Evidence / reproduction:** `command` returns after `write_all`; `MpvFrame` omits `request_id`; errors become generic asynchronous events.
- **Actual / expected:** Callers can receive `Ok` before mpv accepts a command. Commands requiring acknowledgement should resolve against matching response or timeout; truly fire-and-forget commands should say so.
- **Why / root cause:** IDs were added without a pending-request router.
- **Correction / effort:** Dedicated IPC owner, request-id to oneshot map, deadlines, disconnect propagation. Medium-large, 3–5 days.
- **Long-term impact:** Enables deterministic control, retries, and error attribution.
- **Confidence / status:** High — **Confirmed by code inspection**. Cross-examination downgraded High to Medium because no consequential rejection was reproduced.

### P7. Schema migration/version guarantees are decorative

> **Remediation status (2026-07-18): Resolved for schema v1.** Queue, history, playlist, and session loaders now invoke migration/version checks. Pre-versioned object documents are backed up and stamped as v1; future schemas are rejected without rewrite. Service-level v0/v99 tests pass. A future schema v2 still requires a document-specific structural migration before release.

- **Severity / category:** Medium — compatibility, data integrity
- **Affected:** `src/persistence/migrations.rs:27`; queue/history/playlist loaders
- **Evidence / reproduction:** No production caller uses `migrate_in_place`; v0 to v1 writes unchanged JSON and reports success; several loaders accept future versions if fields deserialize.
- **Actual / expected:** Version claims are unenforced. Each document kind should reject future versions, explicitly transform old versions, validate the result, and preserve the source.
- **Why / root cause:** Generic migration infrastructure was introduced before a real migration and never integrated.
- **Correction / effort:** Version-aware store per document kind; delete no-op migration until needed or implement a real v0 path. Medium, 3–5 days.
- **Long-term impact:** A release blocker before schema v2; reduces downgrade/upgrade corruption risk.
- **Confidence / status:** High — **Confirmed by code inspection**. Downgraded from High because no real schema transition exists yet.

### P8. Three accepted configuration options are inert

> **Remediation status (2026-07-18): Resolved by deletion.** `audioOnly`, `resolveBeforePlayback`, and `showFooterHints` were removed because the current product cannot honor meaningful alternatives. Nested config objects now deny unknown fields, and regressions prove these former no-op keys are rejected rather than silently accepted.

- **Severity / category:** Medium — product contract, configuration
- **Affected:** `audioOnly`, `resolveBeforePlayback`, `showFooterHints` in `src/config/model.rs:48`; unconditional behavior in mpv, app, and UI
- **Evidence / reproduction:** Consumer search finds only declaration/default/serialization. mpv always uses `--no-video`, resolution always occurs, footer always renders.
- **Actual / expected:** Valid config changes nothing. Settings should control behavior or be removed/rejected.
- **Why / root cause:** Aspirational schema shipped before runtime wiring.
- **Correction / effort:** Remove unsupported settings or wire end-to-end with behavior tests. 0.5–2 days.
- **Long-term impact:** Restores trust in configuration and prevents migration debt.
- **Confidence / status:** Very high — **Confirmed by code inspection**.

### P9. Help content is clipped and cannot be reached

> **Remediation status (2026-07-18): Resolved.** Help now renders from the command catalog, scrolls at 80x24, exposes the final commands, and restores the view that opened it on Esc/`?`. Reducer and minimum-viewport regressions pass.

- **Severity / category:** Medium — usability, accessibility, content
- **Affected:** `render_help` in `src/ui/views.rs:1204`; Help test in `tests/ui_snapshots.rs:335`
- **Evidence / reproduction:** Executed 100x34 render stopped after `N New playlist`, omitting later playlist/history/message/help/quit rows. Help has no scroll state and Esc does not restore the previous view.
- **Actual / expected:** The main discoverability surface silently hides controls. Help should scroll/search or scope itself to the active view and close normally.
- **Why / root cause:** Static rows grew without a viewport model; tests assert only early substrings.
- **Correction / effort:** Canonical command metadata, Help scroll/search, view scope, completeness tests at 80x24. 0.5–1.5 days.
- **Long-term impact:** Prevents recurring command/help drift.
- **Confidence / status:** Very high — **Confirmed by execution**.

### P10. Mouse hit-testing duplicates renderer geometry

> **Remediation status (2026-07-18): Resolved by renderer contract.** List renderers now publish their actual selectable row rectangle; mouse handling consumes it instead of reconstructing Search/filter/split-pane offsets. Double-click activation requires the same view and item. A UI regression verifies Search's published rows; physical-terminal injection remains outside the deterministic suite.

- **Severity / category:** Medium — interaction correctness
- **Affected:** `handle_mouse` in `src/app/mod.rs:309`; Search table in `src/ui/views.rs:450`
- **Evidence / reproduction:** Source arithmetic maps rows independently from the actual Ratatui layout; rendered coordinates indicate a Search offset. Double-click state stores time but not the original target.
- **Actual / expected:** A click can plausibly select/play another row; two quick clicks on different rows can trigger activation. Hit zones should come from rendered geometry and double-click must match view/pane/item.
- **Why / root cause:** Interaction geometry is duplicated in controller and renderer.
- **Correction / effort:** Publish row/item hit zones from render or extract one shared layout model; add synthetic mouse tests. 1–2 days.
- **Long-term impact:** Prevents every layout change from breaking mouse behavior.
- **Confidence / status:** Row mismatch **Strongly indicated**; target-independent double-click **Confirmed by code inspection**. No physical injection.

### P11. External process output and image decode are unbounded

> **Remediation status (2026-07-18): Resolved.** yt-dlp stdout/stderr are consumed concurrently under 16 MiB/1 MiB limits; limit or timeout kills and reaps the child and returns `ResourceLimit`. Exact-limit and one-byte-over subprocess tests pass. Thumbnail transfer is capped at 5 MiB and decoding uses 4096x4096 and 64 MiB allocation limits; an oversized-dimension regression is rejected.

- **Severity / category:** Medium — reliability, resource exhaustion
- **Affected:** `wait_with_output` in `src/media/yt_dlp.rs:241`; `curl.output` and `image::load_from_memory` in `src/app/mod.rs:1540`
- **Evidence / reproduction:** Complete stdout/stderr/image bytes are buffered; no byte, entry, or decoded-dimension limit exists.
- **Actual / expected:** Memory scales with external output. Operations should stream with operation-specific caps and abort with a structured too-large error.
- **Why / root cause:** Timeouts were treated as resource limits; they do not bound bytes.
- **Correction / effort:** Concurrent bounded pipe readers, playlist caps, image header/decoder limits. Medium, 2–4 days.
- **Long-term impact:** Bounds memory and parser work under large playlists or malformed responses.
- **Confidence / status:** **Strongly indicated**. Cross-examination downgraded High because no realistic OOM was measured.

### P12. yt-dlp positional arguments lack an explicit option boundary

> **Remediation status (2026-07-18): Resolved.** Search pseudo-URLs and all direct video/playlist/details/stream positional inputs now follow `--`. Fake-binary capture proves argument boundaries and that hostile query text remains one `ytsearchN:` argument. URL classification structurally validates scheme and exact supported hosts, rejecting lookalike domains and credential tricks.

- **Severity / category:** Medium disputed — subprocess input boundary
- **Affected:** URL-taking methods in `src/media/yt_dlp.rs:137`, `:178`, `:208`; loose classification in `src/media/import.rs:17`
- **Evidence / reproduction:** Raw import input can reach a positional slot without `--`. Search is safer because input is embedded in `ytsearchN:`; video/mix paths are canonicalized. No benign real-option reproduction was executed.
- **Actual / expected:** Option-shaped import input may alter yt-dlp parsing; intended URL inputs should follow an option terminator and structural URL policy.
- **Why / root cause:** Direct argv avoided shell injection but did not fully define positional boundaries.
- **Correction / effort:** Add `--` immediately before true positional values; replace substring URL recognition with parsing and explicit supported-host policy. Small-medium.
- **Long-term impact:** Hardens the adapter without narrowing supported sources accidentally.
- **Confidence / status:** **Strongly indicated / hypothesis requiring focused verification**. Reviewers disagreed between Low, Medium, and High; current conclusion is Medium hardening risk, not confirmed exploit.

### P13. Debug action logging can expose signed URLs and binary payloads

> **Remediation status (2026-07-18): Resolved.** Action and playback-event debug logs now record only enum discriminants; payload URLs and thumbnail bytes are never formatted. yt-dlp stderr replaces HTTP(S) tokens with `[url redacted]`, with a signed-URL regression test.

- **Severity / category:** Medium — information exposure, observability performance
- **Affected:** `tracing::debug!(?action)` in `src/app/mod.rs:404`; action payloads in `src/app/action.rs`
- **Evidence / reproduction:** Derived `Debug` includes resolved stream URLs and `ThumbnailLoaded.bytes`; the action is cloned and debug-formatted.
- **Actual / expected:** Expiring playback credentials and huge byte arrays can enter logs. Logs should use redacted structured summaries and sizes.
- **Why / root cause:** Whole action objects are treated as safe observability payloads.
- **Correction / effort:** Custom action names/fields, URL redaction, never log bytes. Under 1 day.
- **Long-term impact:** Prevents local secret leakage and pathological log growth.
- **Confidence / status:** High — **Confirmed by code inspection**; actual debug log was not captured.

### P14. `doctor` diagnoses state it has already changed

> **Remediation status (2026-07-18): Resolved.** `doctor` no longer ensures directories, initializes logs, or uses the backup-producing config loader. It reports absent config as defaults-in-use, preserves malformed config without creating `.bak`, continues all dependency checks, includes optional curl, and inspects unwritable directories without a write probe. Five black-box tests run only against disposable roots.

- **Severity / category:** Medium — diagnostics, information integrity
- **Affected:** `src/main.rs:29`, `run_doctor` at `:54`, `ensure_dirs`
- **Evidence / reproduction:** Startup creates directories before `doctor` checks existence; it prints `OK config <path>` even when the file is absent and defaults were used. Malformed config aborts the report. `curl` is used at runtime but not checked.
- **Actual / expected:** “All checks passed” overstates what was observed. Doctor should report absent/default config accurately, test without destructive repair, and separate `--repair` if desired.
- **Why / root cause:** Diagnostic and startup initialization share one path.
- **Correction / effort:** Dispatch doctor first, make checks read-only, add optional capability checks and actionable output. 0.5–1 day.
- **Long-term impact:** Trustworthy support evidence and automation.
- **Confidence / status:** Very high — **Confirmed by execution and code inspection**. Missing config itself is healthy; the wording is false.

### P15. Details failure becomes permanent false loading

> **Remediation status (2026-07-18): Resolved.** Details now have identity-tagged Loading, Loaded, and Failed transitions. Failure renders `Details unavailable` with the actual error instead of an endless spinner; a reducer regression proves the terminal transition.

- **Severity / category:** Medium — async state, status integrity
- **Affected:** `spawn_details_fetch` in `src/app/mod.rs:1512`; Playing view in `src/ui/views.rs:905`
- **Evidence / reproduction:** Background task sends an action only on success; `None` always renders `Loading details...`.
- **Actual / expected:** UI claims work continues after timeout/failure. Model should distinguish Idle, Loading, Loaded, Failed, with retry.
- **Why / root cause:** Async lifecycle is represented by `Option`.
- **Correction / effort:** Typed state and `DetailsFailed` action. Under 1 day.
- **Long-term impact:** Reusable truthful async-state pattern.
- **Confidence / status:** Very high — **Confirmed by code inspection**.

### P16. Playlist import reason metrics are lossy and misleading

> **Remediation status (2026-07-18): Resolved.** Normalization returns typed `SkipReason` values for deleted, private, unavailable, missing ID, and missing title. Counts remain separate through fetch, action, import builder, and review UI; duplicates are independently derived after normalization. Mixed-fixture and deduplication tests cover every category.

- **Severity / category:** Medium — factual integrity, calculation
- **Affected:** `src/media/yt_dlp.rs:50`, `:178`; reducer `src/app/reducer.rs:487`; import summary
- **Evidence / reproduction:** All rejected entries collapse into `skipped`; reducer labels all skipped as unavailable. In the real flow, the separate missing-metadata category is structurally forced to zero.
- **Actual / expected:** Private/deleted/missing-id/missing-title entries receive fabricated category precision. Normalization should return typed skip reasons and totals should derive from them.
- **Why / root cause:** `Option<Track>` loses rejection semantics.
- **Correction / effort:** `Result<Track, SkipReason>` or normalization outcome plus fixture tests. 1–2 days.
- **Long-term impact:** Makes import decisions and support information trustworthy.
- **Confidence / status:** Very high — **Confirmed by code inspection**.

### P17. “Listened” duration measures lifecycle wall time

> **Remediation status (2026-07-18): Resolved.** History uses a `ListeningAccumulator` over positive media-position deltas only while playing. Pause and explicit seek reset the baseline; backward or greater-than-three-second jumps are excluded. The displayed value now means media seconds listened (and therefore follows media position at non-1x speed), verified by pause/seek/jump tests.

- **Severity / category:** Medium — analytics integrity
- **Affected:** `record_current` in `src/app/mod.rs:1415`; history views
- **Evidence / reproduction:** Duration is `started.elapsed()`. Pause time and speed are ignored; repeated playback-restart may reset it.
- **Actual / expected:** UI labels an estimate as listening time. Use bounded positive media-position deltas while playing, or relabel it explicitly.
- **Why / root cause:** Lifecycle timing substituted for playback telemetry.
- **Correction / effort:** Define metric semantics and accumulate position deltas with seek/speed rules. 2–3 days.
- **Long-term impact:** Prevents increasingly misleading analytics.
- **Confidence / status:** Pause/speed defect **Confirmed by code inspection**; restart magnitude requires runtime traces.

### P18. Exact video URLs are routed through search

> **Remediation status (2026-07-18): Resolved.** Exact URLs emit `SubmitExactVideo`/`RunExactVideo` and call `fetch_video`; a reducer regression proves no `RunSearch` effect is produced. The resulting single track retains exact identity before autoplay.

- **Severity / category:** Medium — input integrity, core journey
- **Affected:** `submit_text_query` in `src/app/mod.rs:668`; unused `fetch_video` in `src/media/yt_dlp.rs:136`
- **Evidence / reproduction:** `InputKind::Video` reconstructs a URL and dispatches `SubmitSearch`; search wraps it in `ytsearchN:`.
- **Actual / expected:** Pasting an exact video can yield ranked search results instead of deterministic identity. Use `fetch_video` and play/add the exact result.
- **Why / root cause:** Classification was added without a dedicated exact-fetch effect.
- **Correction / effort:** Add exact-video action/effect and tests. Under 1 day.
- **Long-term impact:** Reliable deep links and CLI `play <URL>` semantics.
- **Confidence / status:** High — **Confirmed by code inspection**; mismatched live result not reproduced.

### P19. Top History uses contradictory definitions and controls

> **Remediation status (2026-07-18): Resolved.** Aggregation exposes completed plays and all attempts separately, sorts deterministically, and keeps total media time. The Top header counts distinct rows, each row labels both metrics, timestamps are explicitly local, and the Top footer no longer advertises unsupported per-entry deletion.

- **Severity / category:** Medium — information integrity, UX
- **Affected:** aggregation in `src/history/service.rs:79`; views/footer around `src/ui/views.rs:711`, `:775`; app key handling
- **Evidence / reproduction:** Completed, skipped, failed, and stopped all increment `plays`; header shows raw history count while rows are distinct tracks; footer advertises delete in a mode that rejects deletion.
- **Actual / expected:** Users see incompatible counts and unavailable action. Define attempt/play semantics, show aggregate row count, and scope hints by mode.
- **Why / root cause:** A new presentation was layered over raw records without a semantic contract.
- **Correction / effort:** Small-medium, under 2 days.
- **Long-term impact:** Makes analytics and controls internally coherent.
- **Confidence / status:** Very high — **Confirmed by code inspection**.

### P20. Search submission destroys editable query context

> **Remediation status (2026-07-18): Resolved.** Submission clones the trimmed editable query instead of taking it, and success/failure headers identify the submitted query so users can refine or retry without retyping.

- **Severity / category:** Low — search UX, recovery
- **Affected:** `src/app/mod.rs:492`; Search rendering
- **Evidence / reproduction:** `mem::take` clears input; completed results show `Results (N)` rather than the submitted query.
- **Actual / expected:** Refinement and error recovery require retyping and result provenance is hidden. Preserve editable/submitted queries separately.
- **Why / root cause:** Input buffer and operation identity are conflated.
- **Correction / effort:** 2–4 hours.
- **Long-term impact:** Better iterative search with no architectural cost.
- **Confidence / status:** High — **Confirmed by code inspection/render output**. Downgraded from Medium in cross-examination.

### P21. Notifications have nondeterministic lifetime

> **Remediation status (2026-07-18): Resolved.** Notifications carry an absolute monotonic expiry: four seconds for information and eight for errors. Tick handling compares elapsed time instead of spinner phase; boundary tests cover both durations.

- **Severity / category:** Medium — status feedback
- **Affected:** notification dismissal in `src/app/mod.rs:180`; creation in reducer
- **Evidence / reproduction:** No timestamp is stored; message clears whenever global spinner frame is divisible by 20. At 500 ms tick, lifetime can range from under 0.5 seconds to nearly 10 seconds.
- **Actual / expected:** Important feedback can vanish immediately or linger. Store created/expires timestamps and use severity-specific duration.
- **Why / root cause:** Spinner phase reused as a timer.
- **Correction / effort:** 2–4 hours.
- **Long-term impact:** Predictable, testable feedback.
- **Confidence / status:** High — **Confirmed by code inspection**.

### P22. Destructive actions have inconsistent safeguards

> **Remediation status (2026-07-18): Resolved for cited surfaces.** Queue and History bulk clears use the existing confirmation flow. Queue item removal advertises and supports one-level `u` undo; playlist deletion retains confirmation. Reducer tests prove clear does nothing before confirmation and undo restores play order.

- **Severity / category:** Medium — UX, recoverability
- **Affected:** Queue/History/playlist delete bindings and handlers
- **Evidence / reproduction:** Playlist deletion confirms; queue/history bulk clear and some item deletions persist immediately with no undo.
- **Actual / expected:** One key can remove curated/history data. Confirm bulk destructive actions and provide undo for frequent single-item removal.
- **Why / root cause:** Confirmation was implemented per feature, not as a product rule.
- **Correction / effort:** 0.5–1.5 days.
- **Long-term impact:** Consistent safety without modal fatigue.
- **Confidence / status:** High — **Confirmed by code inspection**.

### P23. The app can write documents larger than its own read limit

> **Remediation status (2026-07-18): Resolved.** Reads and atomic writes share `MAX_DOCUMENT_BYTES` (16 MiB). Oversized writes fail before replacing the existing document, verified by a regression test.

- **Severity / category:** Medium — persistence limits
- **Affected:** `src/persistence/json_store.rs:15`, `:45`
- **Evidence / reproduction:** Reads reject above 16 MiB; atomic writes have no size guard; dataset and text growth are weakly bounded.
- **Actual / expected:** A successful save can make next load fail. Enforce compatible operation-specific limits and rotate/shard growing data.
- **Why / root cause:** Limit exists only on ingestion.
- **Correction / effort:** Small-medium, 1–2 days.
- **Long-term impact:** Prevents self-created data lockout.
- **Confidence / status:** High — **Confirmed by code inspection**; realistic entry threshold unmeasured.

### P24. Repository orientation and operational documentation are absent

> **Remediation status (2026-07-18): Technically resolved; one owner decision remains.** The repository now includes README/setup/recovery guidance, architecture and checked-in product contracts, contribution gates, validated config example, dependency rationale, pinned toolchain/MSRV, CI, local artifact ignores, and non-publishable Cargo metadata. No remote or license intent exists, so rights were not fabricated: README/CONTRIBUTING explicitly block distribution until the owner chooses a license.

- **Severity / category:** Medium — documentation, AI-agent readiness
- **Affected:** repository root, `Cargo.toml`, pervasive unavailable PRD references
- **Evidence / reproduction:** No README, architecture note, PRD, config example, CI, contribution guide, release guidance, license, toolchain/MSRV declaration, or troubleshooting guide.
- **Actual / expected:** Users and agents must reverse-engineer 9,608 Rust lines and cannot verify PRD claims. Add a concise source-of-truth map and executable commands.
- **Why / root cause:** Rapid feature development outpaced release surfaces.
- **Correction / effort:** 1–2 days for minimum viable docs and CI.
- **Long-term impact:** High leverage for maintainers and AI agents.
- **Confidence / status:** Very high — **Confirmed by inspection**. Cross-examination downgraded High because this appears to be a local 0.1.0 prototype.

### P25. Central orchestration has split ownership and weak boundary tests

> **Remediation status (2026-07-18): Risk reduced, structural debt remains open.** A full command-path test now executes reduction, effects, service dispatch, and durable queue reload. Unchanged list filters and Top-history aggregation are cached with mutation invalidation, and Tokio features/an unused direct dependency were reduced. `App`/reducer/service ownership remains concentrated and is explicitly documented; a safe consolidation still requires the review's estimated multi-week characterization/extraction effort and is not falsely marked resolved.

- **Severity / category:** Medium — architecture, maintainability
- **Affected:** `src/app/mod.rs:39`, `:402`; `src/app/reducer.rs`; `src/app/state.rs`; `tests/ui_snapshots.rs`
- **Evidence / reproduction:** `App` owns heterogeneous concerns; every action is cloned, reduced, effects executed, then interpreted by another dispatcher. No test constructs the full command path. UI “snapshots” assert selected substrings.
- **Actual / expected:** Complete behavior cannot be reasoned about or tested in one place. One command-processing boundary should own transition plus effects behind injected ports.
- **Why / root cause:** Phase-by-phase growth accumulated in the runtime shell.
- **Correction / effort:** Characterization tests first, then incremental extraction of playback, persistence, and task ownership. 2–4 weeks, not a rewrite.
- **Long-term impact:** Without correction, regression risk and merge contention compound with feature count.
- **Confidence / status:** High — **Confirmed by code inspection and history**. File size alone is not the finding.

## 3. Confirmed Failures

Only conclusive execution or direct code facts are included here.

1. Valid JSON with invalid queue indices panics on normal Queue rendering.
2. Delayed stream resolution blocks quit/input processing in the single event loop.
3. Help content is clipped and unreachable at a common 100x34 viewport while its test passes.
4. Startup truncates the log before command dispatch; review executions left it zero bytes.
5. `doctor` prints `OK config` for a nonexistent config path and verifies directories after startup created them.
6. Malformed config makes `doctor` abort with a debug-shaped error, although backup creation succeeds.
7. `play --help` marks query optional, while `play` without a query fails.
8. Home displays `N to create` where Home has no `N` binding.
9. Three accepted config fields have no runtime consumers.
10. Queue/history/playlist migration is not connected, and the v0 migration reports success without updating version.
11. mpv command methods return after socket write and do not correlate responses.
12. Import skip reasons are collapsed and then displayed as precise categories.
13. Details failures have no failure state and leave Loading visible.
14. Exact video URLs are routed through search rather than exact fetch.
15. History “listened” uses elapsed lifecycle wall time, not audible time.
16. Top History mixes attempts with plays and exposes contradictory counts/actions.
17. Default Cargo tests pass while live tests remain ignored; the mpv test can return success without running if dependencies are absent.

## 4. Unverified High-Risk Hypotheses

These are intentionally separate from confirmed failures.

- Option-shaped playlist import input may be interpreted by yt-dlp as an option. A harmless argv/parser reproduction is needed; there is no demonstrated remote or privilege boundary.
- Unbounded yt-dlp/curl/image output may cause practical memory exhaustion. Measure RSS with bounded fixtures before assigning High.
- Multiple application instances may race on `mpv.sock`, fixed `.tmp` files, and last-writer-wins persistence.
- A real mpv crash may leave the application permanently degraded because no restart/reconnect owner exists.
- Large histories/queues may cause input lag because filtering, aggregation, allocation, and rendering scale with total records and redraw after every event.
- Parent-directory durability after rename may be insufficient under abrupt power loss; platform-specific fault testing is required.
- Unknown current yt-dlp availability states may be incorrectly normalized to Available; verify against the installed yt-dlp vocabulary.
- Search mouse row selection may be wrong in a physical terminal; synthetic event injection should settle it.

## 5. Quick Wins (under 30 minutes each)

1. Require at least one CLI `play` query value through Clap.
2. Change Home empty-state text from `N to create` to the actual route, or bind `N` on Home.
3. Stop logging full `Action` values; log action name, IDs, and byte counts only.
4. Replace `File::create` with an append-capable setup as an immediate mitigation; schedule rotation in Phase 1.
5. Print `INFO config: defaults in use; file absent` instead of `OK config <nonexistent path>`.
6. Check/report `curl` as an optional artwork dependency.
7. Add `--` before true positional yt-dlp URL arguments after a compatibility test.
8. Render `Results for “query”` using the already retained submitted query.
9. Correct Help section labels so Queue-only and mode-specific commands are not presented globally.
10. Remove the unexplained imported-playlist sync marker until refresh is reachable.
11. Change generic mix failure notification to an error-severity action.
12. Add an explicit metadata Failed rendering even before adding retry.

## 6. High-ROI Improvements

- Validate all persisted documents at one boundary, beginning with queue invariants. This removes a confirmed crash and prepares migrations.
- Move stream resolution into one supervised playback request pipeline. This fixes the freeze and establishes cancellation/supersession semantics.
- Introduce typed operation states (`Idle/Loading/Loaded/Failed/Cancelled`) for import, details, radio, and playback.
- Make persistence an ordered, acknowledged writer with coalescing and shutdown flush. This improves truthfulness and responsiveness without spawning unordered saves.
- Generate Help/footer hints from canonical key metadata and test every command at the minimum viewport.
- Preserve typed information through transformation: import skip reasons, exact URL identity, history outcome/metric semantics.
- Add a minimal CI gate with fmt, check, clippy, deterministic tests, explicit reporting of skipped integration tests, and one isolated CLI diagnostic test.

## 7. Long-Term Architectural Risks

- **Split command ownership:** reducer, effect executor, and service dispatcher can each own part of one action. Cost compounds as every feature crosses more files and implicit ordering.
- **Concrete runtime shell:** terminal, ratatui geometry, services, mpv, yt-dlp, persistence, and tasks are tied to one object, blocking headless workflow tests.
- **Task lifecycle by convention:** detached tasks will keep producing stale-result bugs unless ownership becomes explicit.
- **Persistence contracts without enforcement:** future schema changes will become increasingly dangerous as user data accumulates.
- **Duplicated geometry:** mouse correctness degrades with every layout change.
- **Fire-and-forget external adapters:** error attribution and recovery worsen as playback behavior grows.
- **Feature breadth before journey completion:** radio, sleep, chapters, refresh internals, and analytics outpace setup, recovery, truthfulness, and tests.

## 8. Missing Test Matrix

| Layer | Missing high-value tests |
|---|---|
| Unit | Queue invariant validation; typed import skip reasons; config-consumer behavior; notification timing; URL parser/argv boundary; Unicode display width; count/date boundaries. |
| Integration | Full command transition through reducer/effects/services; mpv response correlation/rejection; isolated persistence failures; actual migration fixtures; save-then-read 16 MiB boundary. |
| End-to-end | Search/input → exact result → resolve → mpv → queue advance; delayed resolver remains responsive; cancel import and reject late completion; mpv crash/restart; malformed config doctor recovery. |
| Accessibility | 80x24 Help reachability; keyboard-only modal lifecycle; focus restoration; low-color/monochrome; Unicode/CJK/emoji; screen-reader feasibility documentation. |
| Performance | Draw count under mpv events; 1k/10k/100k records; large playlist parse; image decode limits; startup/build/binary measurement. |
| Security | Harmless argv capture; allowed URL schemes/hosts; signed URL log redaction; oversized stdout/stderr/frame/image limits; file permissions. |
| Content validation | Keymap/help/footer parity; empty-state actions exist; CLI usage matches validation; every config field changes behavior; error states include recovery. |
| Data integrity | Schema v0/v1/v99 by document kind; invalid queue order/position; import reason arithmetic; history metric semantics; UTC/local rendering. |
| Failure recovery | Disk full/read-only/rename failure; corrupt files; stale `.tmp`; mpv EOF; yt-dlp timeout/cancellation; missing curl; shutdown with writes in flight; multi-instance race. |

## 9. Information Integrity Report

- **Factually false:** Home `N to create`; import reason breakdown; `OK config` for absent file; migration success without version change.
- **Internally contradictory:** optional CLI query vs required runtime query; Top History counts/actions; Help scopes; IPC comments promise response correlation that does not exist.
- **Technically correct but misleading:** “listened” is elapsed lifecycle time; defaults make absent config usable but doctor implies a file was checked; imported sync marker implies reachable refresh.
- **Insufficiently supported:** unknown availability mapped positively; upload date formatted by string shape; count formatting around one million.
- **Outdated/unverifiable:** dozens of PRD references point to no repository artifact; `with_data_dir` comments claim a CLI option that does not exist.
- **Traceability loss:** yt-dlp entry rejection becomes one skipped count; lifecycle timing becomes listening analytics; exact video identity becomes search ranking.
- **Calculation risks:** pause/speed/seek handling, import category arithmetic, Top History denominator/count, UTC timestamps without local conversion/zone.

## 10. Content Quality Report

- No source-of-truth onboarding, setup, prerequisite, config, storage, recovery, or support content.
- In-app Help is too long for its static container and lacks close/scroll/search behavior.
- Error text reports symptoms (`mpv down`, `yt-dlp down`, raw config errors) without next steps.
- Empty and footer hints are not systematically tied to actual bindings/state.
- Config accepts options users cannot successfully act on.
- Message log lacks timestamps/source context and may miss errors that are silently logged.
- The project name/description can imply a YouTube Music account client, while behavior is anonymous YouTube search/playback plus local library state.
- Content needing complete rewrite: README/setup, config reference, diagnostic output contract, Help command model, import explanation, history metric explanation.

## 11. Documentation Gap Report

Missing or obsolete:

- Setup/install: supported OS/terminals, Rust/MSRV, mpv, yt-dlp, ffmpeg, curl, launch and first playback.
- Architecture: command ownership, external adapters, task lifecycle, persistence document kinds, generated/runtime files.
- Operations: data/config/log locations, log retention, health checks, dependency verification, observability.
- Deployment/release: packaging, license, versioning, CI, release artifacts, upgrade/downgrade policy.
- Troubleshooting: malformed config, corrupt queue/history, missing binaries, mpv crash, network restrictions, cache/log recovery.
- User guidance: keyboard/mouse model, destructive actions, import semantics, radio, resume, analytics definitions.
- Contribution: commands, quality gates, test categories, ignored/live tests, architecture boundaries, dependency policy.
- Recovery: backups, schema incompatibility, failed saves, rollback, multi-instance behavior.

## 12. Technical Debt Register

| Debt | Origin | Current cost | Compounding risk | Strategy | Deadline |
|---|---|---|---|---|---|
| Blocking resolver | Fast path in central effect executor | Core UI freeze | Every external effect can copy the pattern | Supervised playback request | Immediate |
| Missing queue validation | Serde-only load | Confirmed panic | Future schemas/manual corruption | Validate/recover at load | Immediate |
| Split action ownership | Phase-driven feature growth | Hard reasoning/testing | Cross-cutting edits and stale state | One command boundary | Before next major feature |
| Detached tasks | Per-feature spawning | False cancellation/stale work | Process/resource races | Task owner + generations | Before radio/import expansion |
| Unacknowledged persistence | Optimistic side effects | False success | Silent state divergence | Ordered writer and result actions | Before release |
| Decorative migrations | Aspirational architecture | Misleading safety | Upgrade corruption | Per-kind migration fixtures | Before schema v2 |
| mpv fire-and-forget ambiguity | Partial IPC client | Weak error attribution | Recovery/retry impossible | Correlated responses where needed | Before release |
| Inert config | PRD-first schema | User distrust | Compatibility clutter | Remove or implement | Before config docs |
| Static Help | Appended controls | Hidden commands | Worsens each feature | Canonical metadata + scroll | Before external users |
| Lossy content pipeline | `Option`/generic counters | Incorrect explanations | Analytics/import trust erodes | Typed source semantics | Before analytics claims |
| Missing docs/CI | Prototype velocity | Reverse engineering | Agent/contributor regressions | README, architecture note, CI | Before handoff |

## 13. Refactoring and Deletion Opportunities

- Delete or integrate the no-op generic migration path; do not keep a false safety abstraction.
- Remove inert config fields until behavior exists, especially `audioOnly=false` if video is not a product goal.
- Remove unreachable playlist refresh product cues or complete the workflow.
- Narrow public crate surface; make modules/items private unless intentionally supported.
- Delete unused façades/constants/dependencies after consumer verification (`anyhow`/`tokio-util` were reported as candidates; re-check current dirty tree before editing).
- Replace duplicate renderer/mouse geometry with one shared layout/hit-zone model.
- Consolidate reducer/effect/service action ownership before splitting files mechanically.
- Restrict `image` and Tokio features to actual needs after build/runtime measurement.
- Consolidate unavailable PRD references into one checked-in product/architecture source or remove them.
- Rename UI tests if they remain substring render tests; otherwise adopt real golden/semantic snapshots.

## 14. AI-Agent Friction Report

Current AI-agent readiness is poor despite clear Rust module names.

- **Discoverability:** No README, AGENTS file in repo, architecture map, or PRD; source points to an absent authority.
- **Command clarity:** Cargo defaults are discoverable, but no canonical local gate or distinction between deterministic and live tests.
- **Boundaries:** Actions, reducer, effects, service dispatch, and direct state mutation provide competing seams.
- **Naming predictability:** Generally good at module level, misleading at contract level (`snapshots`, `migrations`, response correlation, `doctor`).
- **Feedback quality:** Compiler/test hygiene is strong; central runtime behavior is difficult to instantiate and integration tests can silently skip.
- **Generated/runtime handling:** `target` is ignored; runtime DB is globally ignored rather than repository-documented; no explicit data-root isolation for tests.
- **Hidden conventions:** macOS ProjectDirs, external binaries, curl artwork, Unix socket behavior, PRD expectations, config defaults.
- **Ownership ambiguity:** `AppState` mixes domain/presentation/protocol state; `App` owns almost everything.

Concrete agent-readiness changes:

1. Add README with a five-minute orientation and exact commands.
2. Add `ARCHITECTURE.md` naming one owner for navigation, playback, persistence, and background tasks.
3. Add `--data-dir` or an explicit test root and document that diagnostics must not mutate real user data.
4. Add one `cargo xtask check` or documented command sequence that reports skipped capabilities honestly.
5. Generate key/help metadata from one source and add semantic assertions.
6. Define generated/runtime files and repository-local ignores.
7. Replace unavailable PRD references with links to checked-in requirements.
8. Add fixture-based failure tests so agents get actionable errors instead of relying on real external state.

## 15. Production Readiness Decision

**Ready with minor conditions.**

The two executed High failures and the major runtime/persistence/diagnostic contract gaps are corrected with focused regressions. The release gate and both fake and real adapters pass. Remaining conditions are explicit: choose a license before distribution, run a dependency advisory scan, retain the single-data-root/single-process operating assumption until locking is designed, and do not expand the central command runtime before P25 characterization and extraction.

Completed production gate:

- Fix and regress-test P1 and P2.
- Preserve logs and make doctor non-destructive/truthful.
- Add operation identity/cancellation for playback/import/radio/details.
- Surface persistence failures and validate document versions/invariants.
- Bound external data and verify argv policy.
- Add CI plus setup/recovery documentation.

## 16. Repository Score

| Dimension | Score / 100 | Reason |
|---|---:|---|
| Architecture | 64 | Operation/persistence/IPC boundaries are enforced; central command ownership remains concentrated. |
| Implementation quality | 84 | Strict gates pass and failure/resource contracts are explicit. |
| Testing | 86 | 165 passing checks across unit, CLI, UI, fake and real adapters; platform/fault tooling gaps remain. |
| UX | 82 | Responsive operations, truthful states, scrollable Help, safe deletion, and recoverable search. |
| Accessibility | 68 | Keyboard-first and minimum-viewport Help are covered; screen-reader and physical mouse testing remain absent. |
| Performance | 72 | External resources are bounded and expensive history/filter derivations are cached; large-data budgets remain unmeasured. |
| Security | 76 | Structural URL/argv policy, redaction, strict config, bounds, and file permissions are tested; advisory scan unavailable. |
| Reliability | 84 | Queue validation, supervised tasks, correlated IPC, ordered persistence, recovery, and terminal states are tested. |
| Factual integrity | 90 | Import, history, date, query, config, and doctor claims now trace to tested semantics. |
| Content quality | 84 | Help, status, errors, setup, recovery, and operational guidance are actionable. |
| Documentation | 88 | README, architecture, product contract, config, dependencies, contribution, and CI are checked in. |
| AI-agent readiness | 80 | Exact gates, boundaries, test isolation, runtime files, and known debt are documented. |

**Weighted overall score: 80/100.** Architecture, platform coverage, and security-scan uncertainty prevent a higher release score.

## 17. Remediation Plan

### Phase 1 — Stop the bleeding

| Priority | Task | Owner profile | Dependencies | Validation | Expected outcome |
|---|---|---|---|---|---|
| P0 | Validate queue invariants and recover/backup invalid data | Rust reliability engineer | None | Fixtures for bad order/position/duplicates plus reproduced crash regression | No persisted queue can panic runtime paths |
| P0 | Move stream resolution to one cancellable generation-aware playback request | Async Rust engineer | Request identity design | Delayed mock proves render/input/q remain responsive; stale result rejected | Core UI never freezes on yt-dlp |
| P0 | Stop log truncation; redact action payloads; add retention | Platform/observability engineer | Logging policy | Seed log, launch once in isolated root, verify preservation/rotation/redaction | Diagnostic evidence survives startup |
| P1 | Make doctor read-only and truthful | CLI/operations engineer | Explicit data-root seam | Missing/default/malformed/unwritable dependency matrix | Trustworthy diagnostics with actionable recovery |
| P1 | Surface persistence failures and prevent false success | Rust application engineer | Typed result actions | Read-only/disk-full/rename fault tests | Memory/UI and disk cannot silently diverge |
| P1 | Add import/radio/details cancellation and terminal states | Async Rust engineer | Shared operation model | Delayed cancel/supersede/fail tests | Cancel remains cancelled; loading always terminates |

### Phase 2 — Stabilize the project

| Priority | Task | Owner profile | Dependencies | Validation | Expected outcome |
|---|---|---|---|---|---|
| P1 | Correlate mpv responses and implement disconnect recovery | Media/IPC engineer | Playback request owner | Fake IPC rejection/reorder/EOF plus real crash smoke | Playback controls are attributable and recoverable |
| P1 | Centralize versioned persistence and compatible size limits | Data/persistence engineer | Queue validation | v0/v1/v99 and >16 MiB fixtures per document kind | Safe upgrade/load/save contract |
| P1 | Bound yt-dlp/curl/image resources and finalize URL/argv policy | Security/reliability engineer | Product host/source policy | Harmless argv capture, RSS tests, oversized fixtures | Defined process trust and resource boundaries |
| P2 | Correct import/history/exact-URL information contracts | Product engineer + QA | Typed source data | Mixed import fixtures; pause/speed/seek history tests; exact URL identity | User-visible values mean what they say |
| P2 | Replace static Help with canonical command model | TUI/UX engineer | Keymap metadata | 80x24 completeness, close/scroll/search, scope parity | Every command is discoverable and accurate |
| P2 | Add CI and deterministic workflow tests | Test infrastructure engineer | Explicit test root | Fresh checkout gate; skipped tests clearly reported | Reproducible release signal |
| P2 | Add minimal README/config/troubleshooting docs | Technical writer + maintainer | Stabilized behavior contract | Commands executed from clean checkout | Users/agents can install, run, configure, recover |

### Phase 3 — Strengthen the foundation

| Priority | Task | Owner profile | Dependencies | Validation | Expected outcome |
|---|---|---|---|---|---|
| P2 | Consolidate reducer/effect/service ownership into one command boundary | Principal Rust architect | Phase 1 characterization tests | Headless workflow tests; no dual action dispatch | Local reasoning and safer changes |
| P2 | Separate domain/application state from UI protocol/geometry state | Rust/TUI architect | Command boundary | Domain tests without Ratatui; renderer contract tests | Independent UI and domain evolution |
| P2 | Introduce ordered persistence writer and multi-instance policy | Systems engineer | Persistence contract | Concurrent-instance and shutdown-flush tests | Defined consistency and durability |
| P3 | Virtualize/cached large collections and coalesce redraws | Performance engineer | Bench harness | 1k/10k/100k latency and draw-count budgets | Stable performance as data grows |
| P3 | Narrow dependency features and public API | Rust maintainer | Behavior stabilized | Binary/build comparison, API consumer check | Smaller maintenance/audit surface |
| P3 | Add architecture/decision records and executable content contracts | Tech lead | Final boundaries | Agent dry run completes a change using docs only | Lower human and AI regression rate |

## Appendix A — Independent review summaries

These are summaries of the six reports produced before reviewers saw one another. They preserve scope, checks, failures, risks, gaps, strongest criticism, top findings, limitations, and confidence without repeating every per-finding field already captured above.

### Agent 1 — Principal Software Architect

1. **Scope:** all source/tests/config/history and repository growth; architecture/dependency direction and public API.
2. **Checks:** fmt/check/clippy/tests/doc-tests/tree/metadata/doctor/git history. All compiler gates passed; 90 tests passed; zero doctests.
3. **Confirmed failures:** inert config, disconnected migrations, uncorrelated mpv IDs, self-fulfilling doctor checks, absent PRD.
4. **Major risks:** central `App`, task lifecycle, persistence compatibility, duplicated state and geometry.
5. **Missing tests:** full action pipeline, schema fixtures, mpv rejection, stale operations, shutdown, real snapshots.
6. **Misleading content:** “thin shell,” unavailable PRD, snapshot naming, fake config/migration surface.
7. **Strongest criticism:** architectural vocabulary exists without enforced boundaries.
8. **Initial top ten:** god object; migrations; mpv IDs; state entanglement; config; detached work; doctor; speculative/dead APIs; orientation; missing seam tests.
9. **Not inspected:** live YouTube, long playback, platforms, large data, security beyond targeted review.
10. **Confidence:** High; initially over-severe on file size/schema/mpv, corrected in cross-examination.

### Agent 2 — Staff QA / Destructive Testing

1. **Scope:** event loop, queue/persistence, mpv/yt-dlp, config, UI edge cases and tests.
2. **Checks:** full tests, fmt, Clippy, missing binaries, corrupt queue, delayed resolver, malformed config.
3. **Confirmed failures:** queue panic, blocked quit during resolution, malformed-config doctor abort.
4. **Major risks:** false cancellation, no mpv recovery, hidden save failures, self-unreadable files, Unicode/layout.
5. **Missing tests:** responsiveness, bad queue matrix, IPC rejection/disconnect, persistence failure, large data, keymap.
6. **Misleading content:** IPC/wait/migration claims and snapshot naming.
7. **Strongest criticism:** the event-driven app blocks on its most latency-sensitive operation.
8. **Initial top ten:** freeze; queue crash; import cancellation; mpv acknowledgement; mpv recovery; migrations; persistence truth; size limit; whole-data rendering; Unicode width.
9. **Not inspected:** live network, real mpv crash, disk/full permissions, large benchmarks, Windows.
10. **Confidence:** High for reproduced failures, medium-high overall.

### Agent 3 — Product and UX Critic

1. **Scope:** complete TUI journeys, key/mouse flows, states, snapshots, real adapter smoke paths.
2. **Checks:** full Cargo gates, live yt-dlp, rendered views, CLI/doctor, dependency tree.
3. **Confirmed failures:** blocking resolver, mouse geometry evidence, Help clipping, destructive inconsistency, query clearing, notification timing, import cancellation, inert config, log truncation, endless details loading.
4. **Major risks:** core journey looks hung, wrong mouse target, data removal, weak recovery.
5. **Missing tests:** handle_key/mouse, end-to-end playback, slow resolver, Help minimum size, cancellation, Unicode.
6. **Misleading content:** incomplete Help, dead/irrelevant hints, unexplained sync marker, missing curl/onboarding, product naming scope.
7. **Strongest criticism:** attractive static screens sit on interaction geometry and long-running work that are not first-class state.
8. **Initial top ten:** freeze; mouse mapping; Help; destructive actions; query loss; notifications; import cancel; config; doctor log; onboarding.
9. **Not inspected:** fully isolated interactive run, terminal matrix, real failure modes, package release.
10. **Confidence:** High; mouse row behavior remained strongly indicated rather than executed.

### Agent 4 — Performance, Secure Implementation, Reliability

1. **Scope:** process/input/output boundaries, event/render loop, IPC, persistence, logging, resources, dependencies.
2. **Checks:** targeted source/metadata/tree/status inspection; intentionally no runtime/network work.
3. **Confirmed by code:** blocking resolver, no `--`, queue panic path, unbounded output, redraw behavior, logging payloads, no mpv correlation, synchronous persistence.
4. **Major risks:** subprocess trust/resource boundary and failure isolation.
5. **Missing tests:** argv, URL policy, bounds, process cleanup, render coalescing, cancellation, IPC, crash durability, redaction.
6. **Misleading content:** mpv-exit/correlation/render cadence/thumbnail safety claims.
7. **Strongest criticism:** unreliable external work shares the central event loop and lacks boundaries.
8. **Initial top ten:** option injection; freeze; unbounded memory; redraw; queue panic; logging; tasks; IPC; persistence blocking; broad features.
9. **Not inspected:** runtime, advisories, platform behavior, real adapters, benchmarks.
10. **Confidence:** High on code mechanisms, medium on production severity; cross-examination downgraded option/output claims.

### Agent 5 — Content Accuracy and Information Integrity

1. **Scope:** source-to-storage-to-UI meaning for config, import, history, URLs, availability, dates/counts, docs.
2. **Checks:** full Cargo gates, live yt-dlp, doctor, metadata, history/git/marker searches.
3. **Confirmed failures:** doctor/log behavior, config no-ops, migration gap, import metrics, listening metric, endless loading, history contradictions/timezone.
4. **Major risks:** invalid queue, unknown availability, URL search routing, curl omission, formatting boundaries.
5. **Missing tests:** diagnostic mutation, schema, metrics, exact URL, availability, timezone/count boundaries.
6. **Misleading content:** unavailable PRD, nonexistent data-dir flag, mis-scoped Help, ffmpeg version probe, absent config OK.
7. **Strongest criticism:** precise-looking information contracts lose meaning between layers.
8. **Initial top ten:** doctor/log; schemas; config; import reasons; listened; details; exact URL; availability; Top History; docs.
9. **Not inspected:** real adverse playlists, full terminal, long sessions, current status vocabulary.
10. **Confidence:** High; option-injection severity remained disputed.

### Agent 6 — Content Quality and User Value

1. **Scope:** onboarding, CLI, config, all UI content/help/errors/empty states and maintainability content.
2. **Checks:** full Cargo gates, live yt-dlp, CLI, doctor clean-home attempt, Help render.
3. **Confirmed failures:** Help clipping, doctor false wording, dead Home hint, CLI mismatch, inert config, no docs, absent PRD, unreachable refresh, weak snapshots.
4. **Major risks:** abandonment, false diagnostic confidence, hidden controls, silent failures.
5. **Missing tests:** content contracts, keymap parity, doctor, config behavior, dependency/error guidance, radio/import outcomes.
6. **Misleading content:** repository/CLI/doctor/Home/Help/config/errors/import/message log/PRD surfaces.
7. **Strongest criticism:** feature breadth outran install-configure-learn-recover journey completion.
8. **Initial top ten:** docs; Help; doctor; Home key; config; errors; radio feedback; CLI query; refresh implication; weak snapshots.
9. **Not inspected:** terminal/platform/accessibility matrix, real failure variants, packaging.
10. **Confidence:** High; reviewer later downgraded its own docs/Help severity from High to Medium.

## Appendix B — Cross-examination and disputes

All six reviewers challenged at least three claims. Their strongest joint corrections were:

- File size is not a High finding. The real issue is split ownership and missing tests.
- Schema/migration debt is Medium now and a High blocker before schema v2.
- mpv no-ack is Medium until a consequential rejection is reproduced.
- Missing README/onboarding is Medium for this local 0.1.0 prototype, High only for external release/handoff.
- Help clipping is Medium, not High, despite being executed.
- Unbounded output/image memory is Medium/strongly indicated until measured.
- Mouse row mismatch is strongly indicated, not execution-confirmed.
- Queue crash and event-loop freeze remain High and execution-confirmed.
- Option handling remains disputed: one reviewer retained a potential High, others rated Low/Medium. Current synthesis uses Medium/hypothesis because the reachable path and impact are not proven.
- Doctor/log behavior should be split: log truncation is definite; missing config is healthy but falsely described; directory checks are self-fulfilling; malformed-config handling is poor.

Fixes reviewers warned against:

- Naively spawning resolution creates stale/out-of-order playback unless requests have identity and cancellation.
- Waiting for mpv acknowledgements in the central loop can recreate the freeze.
- Spawning independent saves can reorder state; use one ordered writer.
- Strict schema rejection without migration/recovery can lock users out.
- Silent queue repair can destroy shuffle/current semantics; preserve and report recovery.
- A YouTube-only allowlist may break intended yt-dlp sources; define product policy first.
- Simple stdout truncation can yield misleading partial JSON; abort and return too-large errors.
- Append-only logging avoids truncation but needs rotation and redaction.
- Universal confirmation harms keyboard flow; confirm bulk actions and use undo for frequent item removal.
- Rendering only on a slow tick can add latency; coalesce high-frequency updates but render input/resize/errors immediately.

Additional verification that would settle the remaining disputes:

1. Harmless argv recorder and benign yt-dlp parser test for option-shaped imports.
2. Synthetic crossterm mouse events against deterministic render geometry.
3. Fake mpv server with matching/reordered/error/missing IDs and disconnect.
4. Bounded 10/100/500 MiB stdout/image fixtures with RSS and responsiveness measurements.
5. Explicit isolated data-root tests for doctor, schema documents, save failures, and multi-instance behavior.
6. Controlled playback traces across pause, speed, seek, restart, and skip for history metrics.
7. 1k/10k/100k record render and persistence benchmarks.

## Appendix C — Learning Record

- **Context:** Six-agent read-only adversarial review of a macOS Rust TUI with local application data resolved through `directories::ProjectDirs`.
- **Symptom:** XDG overrides did not isolate diagnostic runs; `doctor` initialized the real application paths and truncated the real log. Two agent prompts were safety-classified, and two later reviewers required replacement/finalization because they did not return promptly.
- **Root cause (rule):** Never assume environment variables isolate platform-native application directories, and never execute a diagnostic command as “read-only” until startup order and resolved paths have been inspected. Multi-agent reviews also need bounded report contracts and an explicit replacement policy.
- **Fix:** Inspect path resolution and command initialization first; require an explicit `--data-dir`/test-root seam before black-box diagnostics; seed and verify disposable paths; cap independent and cross-examination reports; replace blocked/stuck reviewers without treating tool failure as repository evidence.
- **Prevention check:** Before running any CLI review command, answer: “What files/processes can startup mutate before subcommand dispatch, and how is the data root proven disposable?” Before dispatching reviewers, define a maximum report size, stop condition, and failure replacement rule.
- **Tags:** review-process, macOS, ProjectDirs, diagnostics, multi-agent, isolation
