# Architecture

The executable is an event-driven terminal client with four explicit boundaries:

1. `src/app` owns application state, command reduction, background-operation identity, and effect execution.
2. `src/media` and `src/playback` own untrusted external adapters (`yt-dlp`, `curl`, and `mpv`).
3. `src/persistence`, `src/queue`, `src/history`, and `src/playlists` own validated local documents.
4. `src/ui` and `src/input` own terminal rendering and interaction metadata.

## Domain/UI state split (daemon phase 1)

`AppState` is `{ domain: DomainState, ui: UiState }`. The domain half (queue,
playback, playlists, search, import, channel data, health) is what the
planned daemon owns; the UI half (view, focus, selection, overlays, filters,
notifications, thumbnails) stays with the future client. Domain sub-reducers
take `&mut DomainState`; UI-only transitions live in `src/app/reducer/ui`;
per-family coordinators keep the residual cross-half glue explicit. After
each action, `DomainWatermark` derives coarse `DomainEvent`s and
`apply_domain_events` is the single point where UI state reacts to domain
changes — the seam the daemon socket replaces in phase 2. Supervised domain
work lives under `src/app/domain`, which a guard test keeps free of
`ratatui` types. Known phase-2 debts: `ChannelState::return_to` snapshots
client-local navigation, and `service_actions` handlers still run on `App`.
See `docs/superpowers/specs/2026-07-21-daemon-split-design.md`.

## Daemon and client (daemon phase 2)

`ytm daemon` runs the domain core headless: the same action loop with a
Unix-socket server front-end (`src/daemon`, `src/app/daemon_runtime.rs`).
Protocol frames (`src/protocol`) are versioned NDJSON: hello/welcome with a
full domain snapshot, correlated commands/replies, and broadcast events
carrying fresh payloads per `DomainEvent`. The default `ytm` invocation
attaches as a client (`src/client`, `src/app/client_runtime.rs`): a
`DomainMirror` (an actual `DomainState`) hydrates from the snapshot and
events so Phase 1 rendering runs unchanged, while `src/app/client_route.rs`
classifies every action wildcard-free into local UI work, a wire command
with mirror-resolved context, or a guarded refusal. Search and channel
data are per-client: search results return on correlated replies; channel
navigation restoration lives in a client-local snapshot stack. History is
read from the shared store read-only and reloaded on `HistoryChanged`.
Slow clients are dropped at a bounded outbound queue; daemon loss triggers
bounded respawn-and-reattach.

## Runtime data flow

Input becomes an `Action`. The pure reducer updates `AppState` and emits `Effect` values. The app runtime executes effects and sends completion actions back through the same channel. Network/process work never runs inline in the event loop. `OperationRegistry` owns cancellation tokens and join handles; a completion is accepted only when its operation ID is still current.

Track actions are resolved from the active view by `resolve_track_context`, which captures a cloned track plus exact source occurrence. The overlay owns input while open but does not pause playback. Browser and clipboard children use bounded cooperative cancellation, explicit kill-and-reap teardown, and a unique menu generation so an old completion cannot close a newer menu for the same track.

Channel navigation has two supervised operation domains: metadata resolution for legacy tracks and bounded page fetching. `ChannelState` owns newest-first append, cross-page video-ID deduplication, selection, retry, exhaustion, and a navigation snapshot. Completions must match both the active normalized channel URL and requested page; stale results are discarded. A nested channel visit preserves the previous `ChannelState` for Back restoration.

Playback control has two stages. Resolution obtains a temporary stream URL in a supervised task. `PlaybackController` then serializes acknowledged commands to one `mpv` IPC client. `MpvIpc` correlates every response by request ID and fails pending requests on timeout, malformed input, or disconnect. The selected/current track changes only after the matching resolution succeeds.

`Queue::effective_next` is a non-mutating projection of queue position, repeat, and shuffle state. `TrackTransitionState` is keyed by a unique accepted mpv file-load occurrence and requires fresh duration and position data for that occurrence. It fires once inside the final 15-second window, freezes while paused, and is consumed only by the shared bottom-player renderer; rendering has no playback or queue side effects.

## Persistence invariants

- JSON reads and writes share a 16 MiB limit and atomic replacement.
- Every queue index, order entry, cursor, and play-history position is validated before runtime use.
- Schema version 0 is migrated with a backup; version 1 is current; future versions are rejected without rewrite.
- Queue, history, and session writes pass through one ordered/coalescing writer. Shutdown flushes it. Playlist CRUD remains synchronous so UI success follows durable success.
- Persistence failure is observable in the UI; optimistic state is explicitly labeled non-durable.

## External-data invariants

All positional subprocess inputs follow `--`. Supported YouTube URLs are parsed structurally; host suffix lookalikes and credential tricks are rejected. Process output, thumbnails, configuration collection sizes, and persisted files have explicit bounds. Logs record action/event kinds and redact URL payloads.

## UI invariants

The renderer publishes selectable hit rows into state; mouse code does not recalculate layout. Double-click requires the same view and item. Help renders from the command catalog in `src/input/keymap.rs`, scrolls at 80x24, and restores the opening view. Bulk deletion requires confirmation; queue-item deletion has one-level undo.

## Known concentration of ownership

`src/app/mod.rs`, `src/app/reducer.rs`, and `src/ui/views.rs` remain too large. They are not permission for unrelated abstractions: extract a cohesive command processor, playback session, or view module only with characterization tests. See the remediation plan under `docs/superpowers/plans` for measured consolidation work.
