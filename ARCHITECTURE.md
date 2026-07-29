# Architecture

## Workspace layout

The repository is a cargo workspace. The crate graph is the outermost
boundary: it is enforced by the compiler, not by review.

```
crates/
  ratatube-domain     pure core: state, per-context commands, effects,
                      change events, media/queue/playlist/history models.
                      No tokio, no ratatui, no process or filesystem code.
  ratatube-protocol   versioned NDJSON frames and wire DTOs. Depends on
                      domain only, so both runtimes cannot drift.
  ratatube-services   the impure edge: mpv process and IPC, yt-dlp,
                      thumbnails, persistence, clipboard, child processes.
                      Depends on domain and tokio; cannot reach the UI.
  ratatube-ui         UI state, presentation reducers, ratatui rendering,
                      keymap, themes, icons. Reads domain, never writes it.
  ratatube            binary: the daemon runtime, the client runtime, the
                      CLI, and the wiring between the four.
```

`ratatube-domain` cannot render, spawn, or block, because those crates are
absent from its dependency tree. CI asserts that directly
(`cargo tree -p ratatube-domain`), replacing the source-scanning guard test
that used to approximate it. `ratatube-services` and `ratatube-ui` are
siblings: neither can name the other, so a service cannot draw and a widget
cannot spawn a process.

Each service module re-exports the domain model it adapts (for example
`ratatube_services::media` re-exports `Track` next to the yt-dlp client), so
call sites keep one vocabulary per subject. Where a type was half model and
half IO it was split: `HistoryLog` is the domain log the UI renders, and
`HistoryService` is the file-backed store that derefs to it.

Inside the binary crate, the remaining boundaries are:

1. `crates/ratatube/src/app` owns state composition, command reduction, background-operation identity, and effect execution.
2. `app/domain` owns supervised external work (resolution, imports, thumbnails, recovery).
3. `client/` and `daemon/` own the two runtimes and the socket.

## Domain/UI state split (daemon phase 1)

`AppState` is `{ domain: DomainState, ui: UiState }`. The domain half (queue,
playback, playlists, search, import, channel data, health) is what the
daemon owns; the UI half (view, focus, selection, overlays, filters,
notifications, thumbnails) stays with the client. Domain sub-reducers
take `&mut DomainState`; UI-only transitions live in
`crates/ratatube-ui/src/reducer`; per-family coordinators keep the residual
cross-half glue explicit. After
each action, `DomainWatermark` derives coarse `DomainEvent`s and
`apply_domain_events` is the single point where UI state reacts to domain
changes — the seam the daemon socket replaced in phase 2. `DomainState` and
its transitions now live in `ratatube-domain`; supervised external work for
them lives under `crates/ratatube/src/app/domain` in the binary.

Action routing is wildcard-free everywhere it happens — `client_route.rs`,
`service_actions/`, and both reducer trees — so a new variant, including the
by-id ones only daemon clients send, cannot compile until it is given an
owner. Where one enum fans out to several sub-reducers the dispatcher holds
the single enumeration and hands each sub-reducer a narrowed input, so the
"message from another family" case is unrepresentable rather than absorbed by
a catch-all or a panic. Remaining debt: `service_actions` handlers still run
on `App` rather than a domain-owned service type.
See `docs/superpowers/specs/2026-07-21-daemon-split-design.md`.

## Daemon and client (daemon phase 2)

`ratatube daemon` runs the domain core headless: the same action loop with a
Unix-socket server front-end (`crates/ratatube/src/daemon`,
`app/daemon_runtime.rs`).
Protocol frames (`ratatube-protocol`) are versioned NDJSON: hello/welcome with a
full domain snapshot, correlated commands/replies, and broadcast events
carrying fresh payloads per `DomainEvent`. The default `ratatube` invocation
attaches as a client (`crates/ratatube/src/client`, `app/client_runtime.rs`): a
`DomainMirror` (an actual `DomainState`) hydrates from the snapshot and
events so Phase 1 rendering runs unchanged, while `app/client_route.rs`
classifies every action wildcard-free into local UI work, a wire command
with mirror-resolved context, or a guarded refusal. Search and channel
data are per-client: search results return on correlated replies; channel
navigation restoration lives in a client-local snapshot stack. History is
read from the shared store read-only and reloaded on `HistoryChanged`.
Slow clients are dropped at a bounded outbound queue; daemon loss triggers
bounded respawn-and-reattach.

The socket is the single-instance lock, and claiming it is deliberately
conservative. It is published by binding a short staging name, restricting it
to the owner, then hard-linking it into place — atomic like a rename, but it
fails `EEXIST` instead of clobbering a socket a concurrently starting daemon
just published. A probe connect that fails `ECONNREFUSED` means nobody is
listening, so the socket is stale and gets rebound; any other probe error
refuses to start rather than risk displacing a live daemon (a full accept
backlog answers `EAGAIN`, which previously caused socket theft). The
handshake read is bounded, and connected clients are capped, so a peer that
connects and never speaks cannot hold a task and descriptor indefinitely.

## Runtime data flow

Input becomes an `Action`. The pure reducer updates `AppState` and emits `Effect` values. The app runtime executes effects and sends completion actions back through the same channel. Network/process work never runs inline in the event loop. `OperationRegistry` owns cancellation tokens and join handles; a completion is accepted only when its operation ID is still current.

Track actions are resolved from the active view by `resolve_track_context`, which captures a cloned track plus exact source occurrence. The overlay owns input while open but does not pause playback. Browser and clipboard children use bounded cooperative cancellation, explicit kill-and-reap teardown, and a unique menu generation so an old completion cannot close a newer menu for the same track.

Channel navigation has two supervised operation domains: metadata resolution for legacy tracks and bounded page fetching. `ChannelState` owns newest-first append, cross-page video-ID deduplication, selection, retry, exhaustion, and a navigation snapshot. Completions must match both the active normalized channel URL and requested page; stale results are discarded. A nested channel visit preserves the previous `ChannelState` for Back restoration.

Playback control has two stages. Resolution obtains a temporary stream URL in a supervised task. `PlaybackController` then serializes acknowledged commands to one `mpv` IPC client. `MpvIpc` correlates every response by request ID and fails pending requests on timeout, malformed input, or disconnect. The selected/current track changes only after the matching resolution succeeds.

`Queue::effective_next` is a non-mutating projection of queue position, repeat, and shuffle state. `TrackTransitionState` is keyed by a unique accepted mpv file-load occurrence and requires fresh duration and position data for that occurrence. It fires once inside the final 15-second window, freezes while paused, and is consumed only by the shared bottom-player renderer; rendering has no playback or queue side effects.

## Persistence invariants

- JSON reads and writes share a 16 MiB limit and atomic replacement. A write goes to a per-write temp name (never shared, so concurrent writers cannot publish each other's payload), is set owner-only before it is published, is fsynced, is renamed, and then the parent directory is fsynced so the rename itself survives a crash.
- The data directory and the control socket are owner-only, and persisted documents are written `0600` rather than relying on the directory mode alone.
- Every queue index, order entry, cursor, and play-history position is validated before runtime use — on **both** ingress paths. The on-disk path rejects a malformed queue and preserves a `.bak`; the client mirror rejects one arriving from the daemon and keeps the queue and revision it already holds, because those two travel as a pair (an advanced revision with stale content would let the daemon accept an index-based command the client computed against a queue it never received).
- Play-order positions stored in `play_history` are carried through every `order` mutation — insertion, removal, reorder, and shuffle toggles — by track identity, so Previous cannot navigate to the wrong track. Corruption here stays in bounds, so `validate` cannot catch it after the fact.
- Schema version 0 is migrated with a backup; version 1 is current; future versions are rejected without rewrite.
- Queue, history, and session writes pass through one ordered/coalescing writer. Shutdown flushes it. Playlist CRUD remains synchronous so UI success follows durable success.
- Persistence failure is observable in the UI; optimistic state is explicitly labeled non-durable.

## External-data invariants

All positional subprocess inputs follow `--`. Supported YouTube URLs are parsed structurally; host suffix lookalikes and credential tricks are rejected. Process output, thumbnails, configuration collection sizes, and persisted files have explicit bounds. Logs record action/event kinds and redact URL payloads.

## UI invariants

The renderer publishes selectable hit rows into state (list hit area plus window offset, per-item Home zones); mouse code does not recalculate layout. Double-click requires the same view, pane, and item within 500 ms and performs Enter by synthesizing the key through the normal keyboard path. Visual and interaction conventions live in `DESIGN.md`. Help renders from the command catalog in `crates/ratatube-ui/src/input/keymap.rs`, scrolls at 80x24, and restores the opening view. Bulk deletion requires confirmation; queue-item deletion has one-level undo.

## Known concentration of ownership

The former monoliths (`app/mod.rs`, `app/reducer.rs`, and the old `ui/views.rs`) have been split into the `reducer/`, `state/`, `service_actions/`, and `ui/views/` module trees. The largest remaining single files are `app/client_route.rs` (deliberately wildcard-free, so it grows one arm per action) and the two runtime loops (`daemon_runtime.rs`, `client_runtime.rs`). They are not permission for unrelated abstractions: extract a cohesive module only with characterization tests.
