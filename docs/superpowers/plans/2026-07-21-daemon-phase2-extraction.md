# Daemon Phase 2 — Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A working background service: `ytm daemon` runs the domain core headless behind a Unix socket; CLI one-shots (`play`, `pause`, `status`, `quit`) control it; the TUI attaches as a client over a `DomainMirror`. Music survives UI exit.

**Architecture:** The daemon reuses today's `App` event loop headless (no terminal, no render, `picker: Option`): a socket server translates protocol `Command`s into `Action`s on the existing bus, and post-action `DomainWatermark` events broadcast wire snapshots to every client. The TUI client keeps its `UiState` + UI reducer and hydrates a `DomainMirror` (an actual `DomainState`) from snapshots, so all Phase 1 rendering code works unchanged. Single-process mode stays the default until the final task flips `ytm` to client mode.

**Tech Stack:** tokio UnixListener/UnixStream, serde_json NDJSON framing (mirrors the in-repo `MpvIpc` discipline), existing fake-binary test harness.

## Global Constraints

- Merge gate per CONTRIBUTING.md before every commit; run `tests/mpv_ipc.rs` alone if it flakes under load.
- Wire limits: one JSON object per line, max frame 16 MiB; socket `<data-dir>/ytm.sock` mode 0600; lock `<data-dir>/daemon.lock` via `flock` with pid inside.
- `PROTOCOL_VERSION: u32 = 1`; mismatched `hello` gets an error reply then disconnect.
- Single-process TUI behavior must stay green through Tasks 1–4; the client flip lands last.
- Spec: `docs/superpowers/specs/2026-07-21-daemon-split-design.md`. Debts absorbed here: `service_actions` stay on the shared core (acceptable — the daemon runs that core); `ChannelState::return_to` moves client-side during Task 5.

---

### Task 1: `src/protocol` — frames, wire types, codec

**Files:** Create `src/protocol/mod.rs`, `src/protocol/frames.rs`, `src/protocol/codec.rs`; declare in `src/lib.rs`.

**Interfaces (produces):**
- `pub const PROTOCOL_VERSION: u32 = 1;`
- `ClientFrame { Hello { protocol: u32 }, Command { id: u64, command: Command } }`
- `DaemonFrame { Welcome { protocol: u32, snapshot: Snapshot }, Reply { id: u64, result: Result<ReplyBody, String> }, Event { event: WireEvent } }` (serde `#[serde(tag = "type", rename_all = "snake_case")]`)
- `Command`: `PlayQuery{query}`, `PlayTrack{track}`, `PlayPause`, `Stop`, `Next`, `Previous`, `Seek{seconds}`, `Volume{delta}`, `ToggleShuffle`, `CycleRepeat`, `QueueAdd{track, next}`, `QueueRemove{order_index, expected_revision}`, `QueueMove{from, to}`, `QueueClear`, `QueueUndo`, `Search{query}`, `Status`, `Shutdown` (extend in Task 5 with playlist/history/channel commands).
- `Snapshot { queue: WireQueue, playback: WirePlayback, current_track: Option<Track>, playlists: Vec<Playlist>, health: Health }` — `WirePlayback` mirrors `PlaybackSnapshot` fields; `WireQueue` mirrors `Queue` (tracks, order, position, shuffle, repeat). `From<&DomainState>` impl builds it.
- `WireEvent`: `QueueChanged{queue: WireQueue}`, `PlaybackProgress{playback: WirePlayback}`, `TrackChanged{track: Option<Track>}`, `TrackDetailsChanged{details: Option<TrackDetails>}`, `PlaylistsChanged{playlists}`, `HistoryChanged`, `ImportChanged`, `Health{health}` — i.e. `DomainEvent` + payload.
- `codec::write_frame(writer, &frame)` / `codec::read_frame(reader) -> Result<Option<T>>` with the 16 MiB bound and clean oversize/malformed errors.

**Steps:** types + codec → unit tests (serde round-trip every variant; version constant; oversize rejection; unknown-field tolerance via `#[serde(deny_unknown_fields)]` NOT set on daemon-received frames) → gate → commit `feat: protocol frames and ndjson codec for the daemon socket`.

### Task 2: Headless daemon core + socket server

**Files:** Create `src/daemon/mod.rs`, `src/daemon/server.rs`, `src/daemon/lock.rs`; modify `src/app/mod.rs`/`lifecycle.rs` (picker becomes `Option`, construction split into `App::new_headless`), `src/app/runtime.rs` (render/input skipped when headless; socket command receiver joins the `select!`), `src/main.rs` (`ytm daemon` subcommand).

**Design:**
- `daemon::run(config, paths)` acquires the flock (exit quietly if lost), binds `UnixListener` at `ytm.sock` (unlink stale socket first after a failed probe connect), then runs the App loop headless.
- Server task per client: handshake (`hello` → `welcome` + `Snapshot::from(&state.domain)`), then reads `Command` frames → maps to `Action`s → sends on the existing `action_tx` with the reply id recorded; replies sent when the action dispatch returns (commands are fire-and-acknowledge except `Status`, which replies with a fresh `Snapshot`, and `Search`, which replies with results via a per-client oneshot registered in an operation map).
- After each `handle_action`, the runtime hands `(events, &state.domain)` to the server's broadcast hook: each `DomainEvent` becomes a `WireEvent` with payload snapshotted from current state, pushed to every client's bounded (1024) outbound queue; overflow disconnects that client.
- `Command::Shutdown` triggers the existing graceful shutdown path (flush writer, reap mpv), then removes socket + lock.
- Daemon logs to the existing `ytm-tui.log`.

**Steps:** picker-optional refactor (single-process TUI still green) → lock + listener + handshake → command mapping + replies → event broadcast → `ytm daemon` subcommand → integration test: spawn daemon with fake mpv/yt-dlp in a tempdir data-dir, connect a raw `UnixStream`, handshake, `QueueAdd` + `Status`, assert snapshot contents and `queue_changed` event → gate → commit `feat: headless daemon serving the domain core over a unix socket`.

### Task 3: Client connection + CLI one-shots

**Files:** Create `src/client/mod.rs` (connect, handshake, request/reply correlation with timeouts, spawn helper), modify `src/main.rs` (subcommands `play`, `pause`, `status`, `quit`; existing `play` switches to daemon path).

**Design:** `client::connect_or_spawn(paths) -> Connection`: try connect; on failure spawn `current_exe() daemon --data-dir …` detached (new session, stdio → log file), retry ~3 s with backoff. `Connection::request(command) -> Result<ReplyBody>`. One-shots print human output (`status`: track, position/duration, queue length; `play`: resolved title) and exit non-zero on error. `quit` sends `Shutdown` and waits for socket close.

**Steps:** connection module + unit tests (fake daemon socket in tempdir) → subcommands → end-to-end test: `play` auto-spawns daemon (fake mpv), `status` sees the track, `quit` shuts down, lock released → gate → commit `feat: ytm play/pause/status/quit as daemon clients with auto-spawn`.

### Task 4: TUI attaches as a client

**Files:** Create `src/client/mirror.rs`; modify `src/app/runtime.rs`/`action_dispatch.rs` (client mode: domain-family actions serialize to `Command`s instead of local dispatch; completion-style actions never arise locally), `src/ui` untouched.

**Design:** `DomainMirror` IS a `DomainState` hydrated from `Welcome.snapshot` and updated per `WireEvent` (fields with no wire representation keep defaults; the transition animation re-derives from position/duration locally). The TUI runs `UiState` + UI reducers exactly as today; `apply_domain_events` runs on received events. Search becomes request/reply: the client stores results into its mirror's `search` (client-local, per spec). Disconnect shows a banner + three bounded respawn/reattach attempts (reuse the mpv recovery pattern).

**Steps:** mirror + event application tests → command translation table (domain `Action` → `Command`, with a test asserting every domain-mutating action maps or is intentionally daemon-internal) → client-mode runtime path behind `ytm --attach` flag first → snapshot tests for the disconnected banner → flip `ytm` default to client mode with auto-spawn → gate → commit(s).

### Task 5: Round-out and hardening handoff

- Extend `Command` with playlist/history/channel operations; move `ChannelState::return_to` to the client; `doctor` reports socket/lock/pid; README + ARCHITECTURE + PRD updates; multi-client broadcast test (two connections, one mutates, both see `queue_changed`); slow-client disconnect test. Remaining polish rolls into the Phase 3 plan per the spec.

### Task 6: Final verification

- Full merge gate; manual smoke: `ytm daemon` in one terminal, `ytm status`/`ytm play <url>` in another; kill daemon under a live TUI and observe reconnect; note deviations in this file.

## Task 4b blueprint — client TUI runtime (worked out 2026-07-21)

`src/app/client_runtime.rs`: `App::run_client(&mut self, terminal, connection)`.
The client App is built like the TUI App (picker present) but never starts
mpv or the persistence writer; the daemon owns both.

**Connection split:** add `Connection::into_stream()` returning a command
sender (writes `ClientFrame::Command` with fresh ids) plus an
`mpsc::Receiver<DaemonFrame>` fed by a spawned reader task. The UI loop must
never await a round trip.

**Loop arms:** terminal events (existing handlers → `action_rx`), daemon
frames, tick (spinner/notification expiry only — no sleep timer, the daemon
owns it). Frames: `Event` → `client::mirror::apply_event` + selection clamp;
`Reply(Tracks)` → `mirror.search = Results` + selection reset;
`Reply(Error)` → notification. Reader-channel close → disconnect banner +
three respawn/reattach attempts (connect_or_spawn), then manual retry key;
on reattach `apply_snapshot` and continue.

**Action routing** (`fn route(action, state) -> Route { Local, Send(Command), Deferred(&str) }`,
with a test asserting every variant is classified):
- Local: all UI-reducer variants (navigation, help, selection, modals,
  filters, panes, notifications), thumbnail completions
  (`on_thumbnail_loaded` path), `Quit` (running=false; effects dropped —
  client never persists).
- Send, existing commands: PlayPause, Stop, Next/Previous, Seek±5/±30 →
  `Seek`, VolumeUp/Down → `Volume`, ToggleShuffle, CycleRepeat, queue ops
  (selected-context resolved locally against the mirror:
  RemoveSelected → `QueueRemove{order_index, expected_revision}`,
  MoveSelected → `QueueMove`, Clear confirmed → `QueueClear`, Undo →
  `QueueUndo`, AddSelected* → `QueueAdd`), `PlaySelected` in track views →
  `PlayTrack`, playlist load/append/delete/create/save-queue →
  `PlaylistLoad`/`PlaylistDelete`/`PlaylistCreate`/`SaveQueueAsPlaylist`,
  playlist track removal → `PlaylistRemoveTrack`, history clear →
  `HistoryClear`, chapter jumps → compute target locally from mirror
  chapters → `SeekAbsolute`, SeekToFraction → duration × fraction →
  `SeekAbsolute`.
- Send, commands still to add (trivial passthroughs):
  `ToggleMute`, `SpeedUp`/`SpeedDown`/`SpeedReset`, `CycleSleepTimer`,
  `ToggleRadio`, `PlayQueuePosition{position}` (new
  `PlaybackAction::PlayQueuePosition` reduced domain-only),
  `Resume{track, position_seconds}` (→ `ResumeTrack`),
  `SearchExact{url}` (→ `SubmitExactVideo`). Search: client sets
  `mirror.search = Searching` locally and sends `Search{query}`.
- Deferred v1 (notify "not available while attached", client stays behind
  `ytm --attach` until these close): playlist rename/edit-details/reorder
  (need concrete-id daemon actions), URL/JSON import flows (review modal
  needs import state on the wire), channel browsing (per-client channel
  replies), history delete-entry (index race design).

**History view:** client loads `HistoryService` read-only from the shared
file and reloads it on `HistoryChanged` events (daemon writes may lag by
the writer's coalescing window). `get_history_view` over the wire is the
Phase 3 replacement.

**Flip to default:** only after the deferred list above is empty; until
then `ytm` stays single-process and `ytm --attach` opts in.

## Progress notes (2026-07-21)

Tasks 1–3 are implemented and gated; Tasks 4–6 (TUI attach, round-out,
final verification) remain. Real-process smoke verified: `play` auto-spawns
the daemon, the daemon survives the CLI exiting, `status` attaches, `quit`
shuts down and removes socket + pidfile.

Deviations so far:

- **Single instance is socket-as-lock**, not `flock` (no libc-level
  dependency): binding fails while a live daemon owns the socket; a stale
  socket is detected by a failed probe connect and removed. `daemon.pid` is
  a best-effort record for `doctor`.
- **`ytm play` reports the resolved title by polling `Status`** (≤5 s)
  after the acknowledged `PlayQuery`, rather than a dedicated deferred
  reply. `Search` replies are deferred properly (generation-keyed).
- **Auto-spawn is smoke-tested, not cargo-tested**: in `cargo test`,
  `current_exe()` is the test harness, so the spawn path cannot run
  in-process. Covered by the manual smoke above.
- **Unix socket paths inherit the platform `SUN_LEN` (~104 byte) limit**;
  very deep `--data-dir` paths fail with a clear error (same pre-existing
  constraint as `mpv.sock`).
- The daemon process keeps an inert `UiState` (the shared `App` core runs
  headless); eliminating it is Phase 3 tightening, per the Phase 1 debt
  notes.
