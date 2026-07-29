# Workspace Split and Per-Context Messages — Design

Date: 2026-07-29. Status: approved in discussion; awaiting spec review.

## Problem

The knowledge graph (graphify-out/GRAPH_REPORT.md) and the recent
playlist-rename bug point at three coupling problems:

1. **`Action` is a god message.** One enum bridges 29 graph communities
   (betweenness 0.149). Every routing site — `action_dispatch.rs`,
   `client_route.rs` (614 lines), `service_actions/` — must enumerate the
   whole world. The rename bug (348e200) happened because one of three
   parallel enumerations had a catch-all that silently dropped by-id
   playlist variants.
2. **Boundaries are convention, not compiler.** The domain/UI split
   (`DomainState` vs `UiState`) is enforced by one guard test and
   discipline. `service_actions` handlers still run on `App`.
3. **Routing decisions live in three places.** Daemon and client runtimes
   each re-derive "who owns this action" instead of the message types
   carrying that answer.

## Decision

Split the single crate into a cargo workspace and split `Action` into
per-context command enums. Boundaries become unbuildable-to-violate
instead of reviewable. No new runtime dependencies; no actor framework —
`reduce()` stays a pure synchronous function (rejected alternatives:
actor-model rework via kameo/ractor, which dissolves the sync reducer the
test suite depends on; tui-realm, which addresses the view layer where the
coupling is not).

## Crate topology

```
crates/
  ratatube-domain     # DomainState, per-context Cmd enums, pure reducers,
                      # DomainEvent, Effect (as data). Deps: serde,
                      # thiserror, ulid, chrono. NO tokio, NO ratatui,
                      # NO process code.
  ratatube-protocol   # NDJSON frames + wire DTOs + versioning.
                      # Deps: domain, serde.
  ratatube-services   # The impure edge: mpv IPC, yt-dlp, curl/thumbnails,
                      # persistence writer, clipboard, browser. Effect
                      # executors, one per context. Deps: domain, tokio.
  ratatube-ui         # UiState, UiMsg, ratatui rendering, keymap, mouse,
                      # themes, icons. Deps: domain (read-only), ratatui,
                      # crossterm.
  ratatube            # Binary: daemon runtime, client runtime, CLI,
                      # standalone wiring. Deps: all of the above.
```

Compiler-enforced rules that fall out:

- domain cannot render, spawn, or block (no tokio/ratatui in its tree);
  the current guard test is replaced by a one-line CI assert on
  `cargo tree -p ratatube-domain`.
- ui sees only `&DomainState`; `apply_domain_events` is the only legal
  reaction path.
- services cannot reach the UI; protocol is the single wire vocabulary
  linked by both runtimes, so they cannot drift.
- Daemon and client runtimes deliberately stay in the binary crate
  (~450 lines each, shared respawn/reattach); message types already
  encode the client/wire distinction, so separate crates buy nothing.

## Message architecture

Per-context, all in `ratatube-domain`, all serializable (may cross the
wire):

```rust
pub enum PlaybackCmd { Toggle, SeekTo(f64), SetVolume(u8), /* … */ }
pub enum QueueCmd    { Add(Track), Remove { index: usize, revision: u64 }, /* … */ }
pub enum PlaylistCmd { Create(String), Rename { id: PlaylistId, name: String }, /* … */ }
pub enum SearchCmd   { Submit(String), LoadMoreChannel, /* … */ }
pub enum HistoryCmd  { Clear, /* … */ }

pub enum Command {   // the wire envelope — the ONLY thing protocol accepts
    Playback(PlaybackCmd), Queue(QueueCmd), Playlist(PlaylistCmd),
    Search(SearchCmd), History(HistoryCmd),
}
```

Client-local, in `ratatube-ui`, deliberately **not** `Serialize` — cannot
cross the wire by construction:

```rust
pub enum UiMsg { Navigate(View), FocusSearch, OpenSettings, /* … */ }
```

Consequences:

- `client_route.rs` shrinks to only mirror-resolved context ("PlaySelected
  means queue position N per my mirror") — est. ~100 lines.
- The rename-bug class dies: each reducer matches its own enum
  exhaustively; no type contains all actions, so no world-spanning
  catch-all can exist.
- `service_actions/` routing is deleted; effect execution becomes one
  executor per context in ratatube-services (`PlaylistStore::apply`,
  `Player::apply`, …).
- Cross-context flows (play-search-result touches search + queue +
  playback) return follow-up commands: `Effect` grows a
  `Followup(Command)` arm. Explicit, loggable, testable. No context
  reaches into another's state.
- Rejected: one flat `Command` enum without sub-enums (simpler wire shape
  but resurrects the god-match).

## Runtime data flow

```
input ──► UiMsg  ──► ui reducer (client-local, sync)
      └─► Command ─► [attached: NDJSON → daemon] ─► domain reduce() ─► Vec<Effect>
                     [standalone: direct call  ] ─────┘
Effect ─► per-context executor (services) ─► completion Command / DomainEvent
DomainEvent ─► broadcast ─► client mirror ─► apply_domain_events ─► UiState
```

Standalone mode becomes the daemon loop and client loop in one process
sharing a channel instead of a socket — one wiring function, not a third
runtime.

## Wire compatibility

The protocol crate gets its own DTOs mapped from `Command`, with a
protocol version bump. Existing version negotiation + bounded
respawn/reattach handles mismatched peers. Runtime stream URLs remain
never-persisted; the 16 MiB frame bound is unchanged.

## Error handling

- Executors return typed `ServiceError` (mpv/yt-dlp/storage), reduced
  into the existing explicit degraded/non-durable domain state.
- Protocol errors stay connection-level (respawn/reattach).
- `UiMsg` handling is infallible by construction.
- No context can swallow another's errors; none can catch them.

## Testing

- Domain reducer tests run with zero tokio (faster; immune to the mpv
  flake class).
- mpv IPC / fake-process tests move to ratatube-services behind executor
  traits; daemon socket tests stay in the binary crate on real wiring.
- Snapshot tests stay untouched in ui/binary crates (copy is API, per
  DESIGN.md).
- New CI assert: `cargo tree -p ratatube-domain` contains no
  tokio/ratatui.

## Migration — three landable phases

Full verify gate green after each phase; each is a stable resting point.

1. **Split the enum in place** (still one crate): introduce per-context
   enums + `Command`, port the three routing sites, delete catch-alls.
   Mechanical (three-pass perl+compiler sweep). Highest standalone value.
2. **Extract ratatube-domain + ratatube-protocol**: move `DomainState`,
   reducers, effects, frames; wire DTOs + protocol version bump land
   here.
3. **Extract ratatube-services + ratatube-ui**: move executors and
   rendering; delete `service_actions/` routing and the guard test;
   binary crate becomes wiring only.

## Risks

- **Churn**: import paths change across ~250 files in phases 2–3;
  mitigated by the phase boundaries and the mechanical sweep technique.
- **Protocol bump**: a stale running daemon must be restarted once after
  phase 2 (`ratatube quit` + auto-respawn).
- **Snapshot copy sweeps**: none expected (no copy changes), but any
  incidental layout change triggers the usual same-commit sweep.
