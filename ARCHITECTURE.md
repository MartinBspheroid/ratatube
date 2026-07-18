# Architecture

The executable is an event-driven terminal client with four explicit boundaries:

1. `src/app` owns application state, command reduction, background-operation identity, and effect execution.
2. `src/media` and `src/playback` own untrusted external adapters (`yt-dlp`, `curl`, and `mpv`).
3. `src/persistence`, `src/queue`, `src/history`, and `src/playlists` own validated local documents.
4. `src/ui` and `src/input` own terminal rendering and interaction metadata.

## Runtime data flow

Input becomes an `Action`. The pure reducer updates `AppState` and emits `Effect` values. The app runtime executes effects and sends completion actions back through the same channel. Network/process work never runs inline in the event loop. `OperationRegistry` owns cancellation tokens and join handles; a completion is accepted only when its operation ID is still current.

Playback control has two stages. Resolution obtains a temporary stream URL in a supervised task. `PlaybackController` then serializes acknowledged commands to one `mpv` IPC client. `MpvIpc` correlates every response by request ID and fails pending requests on timeout, malformed input, or disconnect. The selected/current track changes only after the matching resolution succeeds.

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
