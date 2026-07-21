# ytm Daemon Split — Design

Date: 2026-07-21. Status: approved direction, pending spec review.

## Context

`ytm-tui` is one process: an event-driven terminal client where input becomes an
`Action`, a pure reducer updates `AppState` and emits `Effect`s, and the runtime
executes effects (mpv IPC, yt-dlp, persistence). Playback dies with the UI.

## Goal

Run playback as a background service. The TUI becomes a control layer; `ytm`
launches the UI and attaches to the running service, starting it if needed.
Music keeps playing after the UI exits.

## Decisions (user-confirmed)

- **Lifetime:** the daemon keeps playing after the last UI detaches. Explicit
  `ytm quit` (or SIGTERM) shuts it down. No idle auto-exit.
- **Split scope:** full service. The daemon owns mpv, queue, playlists,
  history, session, search/yt-dlp resolution, and all persistence.
- **Clients:** multi-client. Any number of TUIs and short-lived CLI clients
  may attach; domain events broadcast to all.
- **Lifecycle management:** auto-spawn. `ytm` starts the daemon transparently
  when the socket is not answering. No launchd/systemd files in this project.

## Non-goals

- Windows support (the repo is already Unix-oriented; the socket is a Unix
  domain socket).
- Remote/TCP control, authentication beyond filesystem permissions.
- A separate `ytmd` binary or protocol crate. One binary, subcommands.
- Config hot-reload in the daemon (restart applies changes).

## Architecture

One crate, three layers:

```
 TUI process                      daemon process
┌───────────────────────┐        ┌─────────────────────────────┐
│ input → UiAction      │        │ socket server (unix, NDJSON)│
│ UI reducer → UiState  │◄──────►│ domain reducer + registry   │
│ render(UiState,       │ conn   │ effects: mpv / yt-dlp /     │
│        DomainMirror)  │        │          persistence writer │
└───────────────────────┘        └─────────────────────────────┘
          src/client + src/ui            src/daemon
                    shared: src/protocol
```

- `src/daemon` — today's domain half of `src/app`: domain reducer,
  `OperationRegistry`, playback session, media tasks, persistence. Plus a
  socket server that accepts clients, applies `Command`s as domain actions,
  and broadcasts domain events.
- `src/protocol` — serde types and framing: `Hello`, `Welcome`+`Snapshot`,
  `Command`, `Reply`, `Event`. `PROTOCOL_VERSION: u32 = 1`.
- `src/client` — connection management for the TUI and CLI subcommands:
  connect, handshake, request/reply correlation, event stream, reconnect,
  auto-spawn. Maintains `DomainMirror`, a client-side copy of daemon state.
- `src/ui` + a slimmed UI reducer — navigation, selection, overlays, list
  filters, and rendering from `(UiState, DomainMirror)`. No domain writes.

## Protocol

Transport: Unix domain socket at `<data-dir>/ytm.sock`, mode 0600.
Framing: newline-delimited JSON, one object per line, max frame 16 MiB
(matches the existing persistence bound). Same request-ID correlation,
timeout, and disconnect discipline as the in-repo `MpvIpc` client.

Client → daemon:

```json
{"type":"hello","protocol":1}
{"type":"command","id":7,"command":{"kind":"queue_add","track":{...},"position":"end"}}
```

Daemon → client:

```json
{"type":"welcome","protocol":1,"snapshot":{"queue":{...},"playback":{...},"playlists":[...],"health":{...}}}
{"type":"reply","id":7,"result":{...}}          // or {"error":"..."}
{"type":"event","event":{"kind":"queue_changed","queue":{...}}}
```

- **Commands** (enumerated in `src/protocol`): play track / play query,
  pause-toggle, seek, volume, next/prev, chapter jump, repeat/shuffle,
  queue add/add-next/remove-occurrence/move/clear/undo, playlist
  create/rename/edit/delete/import/copy-track/remove-track/play-all,
  history delete/clear, search query, channel page fetch, track details,
  get-history-view, status, shutdown.
- **Events** (broadcast): `queue_changed`, `playback_progress`,
  `track_changed`, `track_details`, `playlists_changed`, `history_changed`,
  `health` (mpv/yt-dlp availability, persistence-durability warnings),
  `operation_failed` (surfaced as UI notifications).
- **Per-client replies:** search results and channel pages return only to the
  requester; they are per-query, not shared state.
- **History:** raw entries never cross the wire. `get_history_view` returns
  the computed recent/top rows; `history_changed` tells clients to refetch.
- **Thumbnails:** stay in the UI process (curl fetch of a URL the daemon
  provides in track data). No image bytes over the socket.
- Version mismatch in `hello` → error reply naming both versions, then
  disconnect. Normally impossible: one binary ships both halves.
- Slow-client policy: per-client bounded outbound queue (1024 frames);
  overflow disconnects that client, never blocks the daemon.

## Lifecycle

- **Spawn:** `ytm` tries to connect. On failure it removes a stale socket
  (probe first), spawns `current_exe() daemon --data-dir <dir>` detached
  (new session, stdio to log), and retries connection with bounded backoff
  (~3 s total) before failing with a clear message.
- **Single instance:** `<data-dir>/daemon.lock` held with `flock` for the
  daemon's lifetime; the pid is written for `doctor`. Losing the flock race
  means another daemon owns the data dir — exit quietly and let the client
  connect to it.
- **Shutdown** (`ytm quit`, `shutdown` command, SIGTERM): stop accepting
  connections → cancel supervised operations → flush the ordered persistence
  writer → kill-and-reap mpv → remove socket and lock.
- **Daemon loss under a live UI:** disconnected banner in the TUI, then three
  bounded respawn-and-reattach attempts (the existing mpv reconnect policy).
  Manual retry key after that.
- `resumeOnLaunch` moves to daemon startup; `ytm --resume` becomes a client
  request.

## CLI surface

| Command | Behavior |
| --- | --- |
| `ytm` | Launch TUI; auto-spawn daemon; attach |
| `ytm daemon` | Run the service in the foreground (what auto-spawn executes) |
| `ytm play <query-or-url>` | Auto-spawn, send play command, print resolved title, exit |
| `ytm pause` / `ytm stop` / `ytm status` | Short-lived client commands |
| `ytm quit` | Shut the daemon down cleanly |
| `ytm doctor` | Existing checks + daemon/socket/lock state (still read-only) |

## Ownership moves

| Concern | Before | After |
| --- | --- | --- |
| mpv, yt-dlp | app runtime | daemon |
| queue/history/session/playlists persistence | app runtime | daemon only |
| search + channel paging | app runtime | daemon (per-client replies) |
| thumbnails (curl) | app runtime | UI process |
| clipboard, open-in-browser | app runtime | UI process (needs the user session) |
| config.json | read once | read by both at startup; daemon owns limits |
| ytm-tui.log | one process | daemon keeps it; UI writes ytm-ui.log (same rotation) |

## Error handling

- Every daemon-side operation failure becomes an `operation_failed` event
  with the same redaction rules as today's logs; the UI renders it through
  the existing notification slot.
- Persistence failures broadcast in `health` so every attached UI shows the
  existing "changes not durable" warning.
- The UI never blocks on the socket: commands are fire-with-timeout exactly
  like mpv commands today, and the render loop reads only local state.

## Testing

- Protocol: serde round-trips for every frame; version-gate test; oversized
  frame rejection.
- Daemon: integration tests over real local sockets in `--data-dir` tempdirs,
  using the existing fake mpv/yt-dlp binaries; multi-client broadcast
  ordering; slow-client disconnect; single-instance lock race.
- Client: auto-spawn, reconnect-after-kill, stale-socket cleanup.
- UI: snapshot tests for the disconnected banner and unchanged views over a
  `DomainMirror` fixture.
- Existing gate unchanged: fmt, check, clippy `-D warnings`, tests,
  `git diff --check`.

## Phasing (three sub-projects, each shippable)

1. **In-process split.** Separate domain vs UI actions/state/reducer with a
   channel boundary inside the current process. No behavior change; pure
   characterization-tested refactor. Also delivers the `app/mod.rs` /
   `reducer.rs` size remediation ARCHITECTURE.md already calls for.
2. **Daemon extraction.** `src/protocol`, socket server around the domain
   half, `DomainMirror` client in the TUI, auto-spawn, `daemon` / `quit` /
   `status` subcommands. After this phase the feature works end to end.
3. **Hardening.** Multi-client tests, reconnect UX, `doctor` integration,
   `ytm play/pause/stop` one-shots, README + ARCHITECTURE + PRD updates.

Each phase gets its own implementation plan under `docs/superpowers/plans/`.

## Risks

- **Phase 1 is the bulk of the work.** The reducer mixes UI and domain state
  today; the split must be guarded by characterization tests before moving
  code.
- **Progress-event chatter:** `playback_progress` fires ~1/s per client;
  trivial bandwidth, but the mirror must coalesce so the UI never re-renders
  more than once per tick.
- **Two processes, one config:** divergence is possible until restart;
  accepted (documented) rather than solved with hot-reload.
