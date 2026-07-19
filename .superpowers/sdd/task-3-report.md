# Task 3 Report: Render and Operate the Non-Blocking Context Menu

## Status

Implemented and committed as `1fc0b9fcd43a5b20ddf53f809a1c499a902bc8d7`
(`feat: add non-blocking universal track menu`).

Task 3 now provides:

- A bounded, centered, sanitized track context menu rendered after main content
  and before notifications.
- Exact ordered action labels and warning-colored queue/playlist removals.
- Modal-first `j`/`k`/arrow, Enter, and Esc routing while playback state remains
  unchanged.
- `c` opening on Search, Queue, Playlist Detail, History Recent/Top, Now Playing,
  and Home Recent.
- Typed dispatch into playback, queue, playlist, and navigation domains.
- `NavigationAction::VisitChannel(Track)` as the Task 5 handoff.
- A small selected-track details modal that never replaces current playback
  ownership and reuses extended details only when the selected/current IDs match.
- Validated browser and clipboard URL boundaries.
- A native clipboard adapter using direct process arguments, URL bytes on stdin,
  a two-second bound, kill/wait cleanup, and success only after exit status zero.
- Exact queue and playlist occurrence removal with identity revalidation directly
  before mutation.

## TDD Evidence

### Input RED

`cargo test --locked track_context_menu` initially failed both new input tests:

- `c` produced no `OpenTrackContext` action in track-bearing views.
- An open context menu still allowed `q` to route globally.

The first draft used an unbounded `Receiver::recv()` and exposed a test lifecycle
bug. The surviving Cargo/test PIDs were terminated precisely, then every receive
was bounded to 100 ms and every per-view lifecycle to one second. The exact
formerly hanging test subsequently passed in 0.01 seconds.

### Clipboard RED

`cargo test --locked clipboard` failed four tests against the intentionally
unimplemented adapter: exact stdin, pre-spawn URL rejection, non-zero exit, and
timeout cleanup.

The timeout fake now spins inside the fake executable itself, avoiding a nested
`sleep` child and making kill/wait lifecycle verification deterministic.

### Renderer RED

`cargo test --locked --test ui_snapshots context_menu` failed both new snapshots
because `Play now` and all other menu rows were absent. After implementation, the
snapshots verify exact action order, conditional removals, title sanitization, and
the yellow `!` removal marker.

### Dispatch RED

`cargo test --locked context_menu` failed to compile for the missing typed picker,
Visit Channel, Show Details, and selected-track details state. The green behavior
dispatches the captured `Track`, closes successful actions, keeps browser/clipboard
failures open, and cancels stale removals safely.

## Details Ownership Decision

The existing extended-details reducer intentionally accepts metadata only when it
belongs to `current_track`. Reusing it for arbitrary selected tracks would have
blurred playback ownership and stale-operation checks. Task 3 therefore uses the
brief's approved smallest separate details modal:

- It always renders stable fields from the selected `Track`.
- It reuses `current_details` only when the selected track ID equals the current
  track ID.
- It does not start a second metadata-fetch lifecycle.
- It does not alter `current_track`, `current_details`, `details_status`, or
  playback status.

## Removal Revalidation

Queue removal carries `{ order_index, expected_track, expected_revision }`. The
reducer first rejects any queue generation change, then resolves the current
play-order occurrence and compares the full `Track` immediately before `remove_at`.

Playlist removal carries
`{ playlist_id, track_index, expected_track, expected_revision }`. The service
first rejects any playlist collection generation change, then resolves the
playlist by stable ID, converts the current stored occurrence to `Track`, compares
it immediately before mutation, and rolls back if persistence fails.

Both domains have stale duplicate-occurrence cancellation tests and successful
exact-occurrence removal tests.

## Verification

Focused gates:

```sh
cargo test --locked track_context_menu
cargo test --locked clipboard
cargo test --locked context_menu
cargo test --locked --test ui_snapshots
cargo test --locked app::tests::browser
```

Results:

- `track_context_menu`: 9 focused unit tests plus 2 matching UI snapshots passed.
- `clipboard`: 5 matching tests passed, including the app failure-retention path.
- `context_menu`: 10 focused unit tests plus 3 matching UI tests passed.
- UI integration target: 36 passed, 0 failed.
- Browser safety: 1 passed, 0 failed.

Full completion gate:

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Result: all commands exited zero.

- Library/unit tests: 203 passed, 0 failed.
- CLI integration tests: 6 passed, 0 failed.
- mpv IPC integration: 1 passed, 0 failed.
- UI snapshots: 36 passed, 0 failed.
- Mocked yt-dlp integration: 12 passed, 0 failed.
- Live network yt-dlp tests: 3 intentionally ignored.
- Strict all-target Clippy passed with warnings denied.
- Formatting and whitespace checks passed.

Structural gates:

```sh
changed_rust=$(mktemp)
{ git diff --name-only -- '*.rs'; git ls-files --others --exclude-standard -- '*.rs'; } \
  | sort -u > "$changed_rust"
while IFS= read -r file; do
  lines=$(wc -l < "$file")
  test "$lines" -le 250
done < "$changed_rust"

find src/app -type f -name '*.rs' -exec wc -l {} \; \
  | awk '$1 > 250 { print; failed=1 } END { exit failed }'
```

Result: both exited zero with no oversized file output. The legacy UI, keymap,
application input, and UI snapshot files were split by responsibility; every
touched/new Rust file is at most 250 lines.

The post-test process check found no surviving focused Cargo test, test binary, or
fake clipboard process.

## Concerns and Follow-up

- Task 5 still owns channel metadata resolution and actual Channel navigation;
  Task 3 intentionally emits only `VisitChannel(Track)`.
- Selected-track details intentionally show stable local fields for non-playing
  tracks instead of introducing a second metadata fetch lifecycle.
- `c` now owns context-menu opening in Queue and History. Their conflicting clear
  shortcuts moved to documented uppercase `C`; other quick shortcuts are unchanged.

No blocking Task 3 concerns remain.

## Learning Records

### Learning: Bound action-channel waits in input tests

- **Context:** Async modal input tests waiting for an action that was intentionally
  absent during RED.
- **Symptom:** The focused test binary survived for more than four minutes after
  its runner was interrupted.
- **Root cause:** A test that expects routing output must never use an unbounded
  channel receive; feature absence turns the intended assertion failure into a
  process hang.
- **Fix:** Bound each receive with `tokio::time::timeout`, add an outer per-view
  lifecycle timeout, and terminate only the identified stale Cargo/test PID pair.
- **Prevention check:** Every async input test must convert a missing action into a
  bounded assertion failure and leave no test process after interruption.
- **Tags:** rust, tokio, tests, process-lifecycle

### Learning: Keep old boundary tests away from newly active side effects

- **Context:** Task 2's Submit intent became operational in Task 3.
- **Symptom:** The first full suite selected the last menu action (`Copy URL`),
  touched the real clipboard, and failed an assertion that the menu stayed open.
- **Root cause:** A boundary test retained no-op assumptions after the intent
  acquired service behavior.
- **Fix:** Move selection back to deterministic `Play now`, assert typed dispatch
  and closure, then reopen and test close separately.
- **Prevention check:** When an intent changes from model-only to operational,
  audit every old test for accidental selection of external side effects.
- **Tags:** rust, tests, side-effects, task-boundary

## Review Remediation (2026-07-19)

All findings from `review-a7c9c0c..1fc0b9f.diff` were fixed in one follow-up wave.

### Modal capture and atomic routing

- `AppState::modal_capture` is now the single topmost-modal predicate for context,
  selected-track details, notification log, search details, import, confirmation,
  playlist editor, prompt, and picker overlays.
- Keyboard, mouse, and bracketed paste all consult that same predicate before any
  background routing. Context and details overlays therefore block scroll, click,
  tab navigation, seek, and double-click playback actions.
- Lowercase `c` opens the context menu synchronously inside key handling. The next
  event in a rapid `c,q` burst sees modal state and cannot queue Quit.
- Context-to-picker and context-to-details submissions perform a synchronous state
  replacement with no channel round trip or await between closing one modal and
  opening the next.
- Playback event handling is unchanged and continues while overlays are open.

Deterministic evidence includes `rapid_context_then_quit_is_captured_before_quit_can_queue`,
`context_menu_replaces_itself_atomically_with_picker_or_details`,
`context_and_details_modals_block_all_background_mouse_actions`, and
`paste_routes_only_to_the_topmost_prompt_modal`.

### Bounded external process lifecycle

- Browser and clipboard commands now share the Tokio-process implementation in
  `src/platform/child.rs`; no synchronous polling sleep runs on the application
  runtime thread.
- One deadline covers URL validation, adapter selection, spawn, optional stdin
  write/shutdown, stderr draining, and exit wait. URLs above 2,048 bytes are
  rejected before spawn, and stderr capture is capped at 8 KiB.
- Every started-process timeout or lifecycle error kills and waits/reaps the child.
  Nonzero status is an error, stderr is surfaced within its cap, and only status
  zero reports success.
- Browser and clipboard work is owned by the operation registry and completes via
  a typed action, leaving the terminal event loop responsive. A context menu closes
  only after a matching zero-exit completion; spawn, timeout, nonzero, validation,
  and stale-target failures leave it open.
- Linux adapter order follows the active display session: Wayland uses `wl-copy`,
  X11 uses `xclip`, and a dual-session environment tries Wayland then X11. Any
  unavailable or session-incompatible first adapter falls through to the next
  suitable candidate within the original deadline.

The fake clipboard/opener timeout tests write `$$` to a file, `exec sleep 30` so
the recorded PID is the process under management, await the bounded error, and
assert with `ps` that the PID no longer exists. Nonzero tests make the same reaping
assertion. No test uses an unbounded channel receive.

### Stable occurrence identity and view routing

- Context resolution captures a queue or playlist collection revision beside the
  exact occurrence. Every queue membership/order mutation and every stored playlist
  membership/order mutation invalidates the relevant revision.
- Removal requires both the captured revision and full track value to match.
  Reordering exact clones therefore cancels instead of deleting whichever clone
  later occupies the captured index.
- Queue and playlist exact-clone tests both cover stale reorder rejection. The
  playlist test also reloads persisted data and proves both tracks remain stored.
- Ultra-wide Now Playing resolves the selected row as a Queue occurrence whenever
  `PlayingPane::Queue` owns focus, yielding Queue actions instead of current-track
  actions.

### Shortcut and dispatch precision

- Lowercase `c` remains the universal context shortcut. Queue and History clear use
  uppercase `C`, with matching footer and Help text.
- Keymap regressions assert exact `ClearQueue`, `ClearHistory`, and
  `OpenTrackContext` variants. Menu dispatch tests assert the exact typed variant
  and captured track for every action; `VisitChannel(Track)` remains a deferred
  intent with no Channel fetch or view.

### Review-fix verification

Focused commands and results:

```sh
cargo test --locked track_context
cargo test --locked clipboard
cargo test --locked browser
cargo test --locked uppercase_c_clears_queue_and_history_while_lowercase_opens_track_actions
cargo test --locked --test ui_snapshots queue_and_history_footers_document_uppercase_clear_shortcut
```

- Track context: 33 library tests and 3 matching UI tests passed.
- Clipboard: 7 matching tests passed, including fallback, URL bound, exact stdin,
  nonzero reaping, and timeout kill/reap.
- Browser: 8 matching tests passed, including success, spawn failure, nonzero
  reaping, timeout kill/reap, and background completion.
- Shortcut action and footer regressions: 1 + 1 passed.

Final commands:

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Final result: all commands exited zero. The locked suite passed 280 tests with
0 failures; 3 live-network yt-dlp tests remained intentionally ignored. The
breakdown was 224 library tests, 6 CLI tests, 1 mpv IPC test, 37 UI snapshots, and
12 mocked yt-dlp tests. Strict all-target Clippy passed with warnings denied.
Formatting and whitespace checks passed. The touched/new Rust line gate passed;
the largest touched file is 236 lines, below the 250-line limit.

### Original hanging-test root cause and prevention

- **Context:** The original RED input test expected an action that did not exist
  yet and then waited on its channel.
- **Symptom:** Cargo and the test binary survived after the interrupted run instead
  of producing a failing assertion.
- **Root cause:** `Receiver::recv().await` had no deadline. Missing dispatch was a
  valid RED outcome, but the test encoded it as an infinite wait rather than a
  bounded failure.
- **Fix:** Every action receive now uses `tokio::time::timeout`; negative routing
  assertions use `try_recv`; process waits have explicit deadlines; timeout tests
  record and verify child termination/reaping.
- **Prevention check:** Any async test waiting for an event, action, or process must
  define the maximum wait and assert cleanup after the timeout path.

### Learning: Separate completion and timeout test budgets

- **Context:** Fake browser and clipboard commands run concurrently with the full
  Rust test suite.
- **Symptom:** A fake clipboard that exits zero passed focused runs but once hit its
  one-second deadline; on a later full run, a timeout fake was killed before its
  shell received enough CPU to write the PID evidence file.
- **Root cause:** Both normal completion and PID-observable timeout tests had too
  little scheduling margin for the full parallel suite. A bounded test still needs
  enough time for its child to start and publish the evidence being asserted.
- **Fix:** Normal success/nonzero/fallback fakes use a bounded five-second budget,
  while intentional timeout fakes use a three-second deadline and PID reaping
  assertions. CPU-spinning fakes were replaced with same-PID `exec sleep 30`.
- **Prevention check:** Keep every wait bounded, but use short deadlines only when
  timeout is the behavior under test; use low-CPU, same-PID fakes for lifecycle
  assertions.

### Learning: Avoid recursive async dispatch for atomic state replacement

- **Context:** Context-menu submission needed to open picker/details without an
  event-queue gap.
- **Symptom:** Calling the full async dispatcher recursively produced Rust error
  E0733 because the resulting future would have recursive size.
- **Root cause:** Atomic UI state replacement is a synchronous state transition,
  not a second top-level action-dispatch lifecycle.
- **Fix:** Shared `AppState` helpers replace context state directly with picker or
  details state before the handler yields.
- **Prevention check:** When one modal atomically replaces another, use a small
  synchronous transition helper; reserve queued dispatch for independent work.

## Blocking re-review lifecycle fixes

- External-command targets opened from a track menu now carry the menu's unique
  monotonic generation. Successful completion closes the menu only when both the
  track ID and generation still match, so an old command for track A cannot close
  a newly reopened menu for the same track.
- Superseded external commands now receive cooperative cancellation instead of an
  immediate task abort. Cancellation is handled inside the shared child lifecycle,
  which kills and waits for every started process before returning. The operation
  registry retains teardown handles and shutdown waits for them within its bound.
- Regression tests cover same-track close/reopen ownership, cooperative completion
  after rapid replacement, and a real slow child whose PID is confirmed absent
  after cancellation.

Verification executed after the fixes:

```sh
cargo test replacing_external_command_allows_cooperative_teardown_to_finish
cargo test stale_success_does_not_close_reopened_menu_for_the_same_track
cargo test cancellation_kills_and_reaps_started_child_before_returning
cargo test --all-targets
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

All commands exited zero. The full all-target suite passed 283 tests with zero
failures; 3 live-network yt-dlp tests remained intentionally ignored. Clippy
passed with warnings denied. Every touched/new Rust file is at most 250 lines;
the largest is `src/app/operations.rs` at 244 lines.

### Learning Record: cancellation must stay inside child ownership

- **Context:** Rapidly replacing browser or clipboard commands while a child is running.
- **Symptom:** Aborting the outer task bypassed the explicit child wait/reap path.
- **Root cause (rule):** Never race cancellation outside a resource-owning future when
  cancellation cleanup requires awaiting that resource.
- **Fix:** Route cancellation into `run_before`, then kill and wait before it returns;
  retain superseded task handles until cooperative teardown completes.
- **Prevention check:** Every cancellable child-process test must record a real PID,
  cancel through the production token, and assert the PID no longer exists.
