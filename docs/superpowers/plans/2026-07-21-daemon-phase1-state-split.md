# Daemon Phase 1 — In-Process Domain/UI Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `AppState`, the reducer, and the service handlers into a domain half (future daemon) and a UI half (future client), connected by an explicit `DomainEvent` seam — with zero behavior change.

**Architecture:** `AppState` becomes `{ domain: DomainState, ui: UiState }`. Domain sub-reducers take `&mut DomainState`; UI transitions move to a UI reducer taking `&mut UiState` (+ `&DomainState` read-only). Domain mutations emit `DomainEvent`s; a single `apply_domain_events` function is the only place UI state reacts to domain changes. Phase 2 will replace the in-process event `Vec` with the socket broadcast.

**Tech Stack:** Rust 1.88 (pinned), tokio, existing test suites. No new dependencies.

## Global Constraints

- Merge gate before every commit: `cargo fmt --all -- --check`, `cargo check --locked --all-targets`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `git diff --check`.
- **Zero behavior change.** Existing tests are the characterization suite (~35 reducer tests in `src/app/reducer/tests/`, ~35 in `src/app/track_context/tests/`, ~26 App-level in `src/app/tests/` + `src/app/channel/tests.rs`, 43 UI snapshots). They may be mechanically updated for new paths (`state.queue` → `state.domain.queue`) but never for new expectations.
- `DomainState` and everything under `src/app/domain/` must not reference `ratatui` (guard test in Task 6).
- Dependency direction: UI reads domain; **UI never mutates domain** except by dispatching `Action`s. No function takes `&mut DomainState` and `&mut UiState` unless it is the top-level router or `handle_action`.
- Run `tests/mpv_ipc.rs` alone if it times out during a full parallel run (known flake under load).

## Field partition (from the 2026-07-21 src/app inventory)

**DomainState** (`src/app/state/domain_state.rs`): `mpv_ready`, `yt_dlp_ready`, `search: SearchState`, `search_generation`, `input_kind`, `queue`, `queue_revision`, `removed_queue_item`, `playlists`, `playlists_revision`, `channel: Option<ChannelState>`, `import: Option<ImportState>`, `playback: PlaybackSnapshot`, `current_track`, `track_transition`, `playback_occurrence`, `playback_loaded_occurrence`, `position_occurrence`, `duration_occurrence`, `playback_resolution`, `current_details`, `details_status`, `radio`, `radio_operation`, `sleep_timer`, `pending_resume`, `activity`, `resume_points`.

**UiState** (`src/app/state/ui_state.rs`): `running`, `icon_mode`, `view`, `help_return_view`, `help_scroll`, `focus`, `search_input`, `search_detail_open`, `search_thumbnail_track_id`, `search_thumbnail`, `selected_index`, `list_state`, `table_state`, `spinner_frame`, `main_area`, `list_hit_area`, `screen_area`, `selected_playlist`, `prompt`, `playlist_editor`, `confirm`, `picker`, `track_context_menu`, `track_details_modal`, `track_context_generation`, `list_filter`, `visible_indices`, `history_view_mode`, `notification_log`, `show_notification_log`, `notification`, `thumbnail`, `now_playing_scroll`, `now_playing_show_description`, `playing_pane`, `home_section`, `home_recent_len`, `history_len`.

Known frictions (do not "fix" in Phase 1): `ConfirmState.action: Box<Action>` keeps working because `Action` stays unified; `history_len`/`home_recent_len` remain UI mirrors set before render; `track_context_generation` stays UI-owned and travels in action payloads as today.

---

### Task 1: Split `AppState` into `domain` + `ui` containers

**Files:**
- Create: `src/app/state/domain_state.rs`, `src/app/state/ui_state.rs`
- Modify: `src/app/state/app_state.rs` (struct becomes the two-field composite; keep revision helpers, moving each onto the half that owns its fields)
- Modify (mechanical sweep, compiler-driven): everything that reads `AppState` fields — `src/app/**`, `src/ui/**`, `src/input/**`, `tests/ui_snapshots/**`

**Interfaces:**
- Produces: `pub struct AppState { pub domain: DomainState, pub ui: UiState }`, both halves `Default`, field-for-field per the partition table above. All existing helper methods keep their current names, hosted on whichever half owns their fields (`AppState` keeps thin delegating wrappers where call sites are numerous, e.g. `notify()`, `resolve_index()`).

- [ ] **Step 1:** Write the two structs (move field declarations verbatim from `app_state.rs`, with their doc comments) and the composite `AppState`.
- [ ] **Step 2:** `cargo check --locked --all-targets` and fix every access site mechanically: domain fields become `state.domain.*`, UI fields `state.ui.*`. In `src/ui/**` renderers, destructure once at the top (`let AppState { domain, ui } = state;`) instead of sprinkling paths.
- [ ] **Step 3:** Full gate. Every test must pass with only path updates.
- [ ] **Step 4:** Commit `refactor: split AppState into DomainState and UiState`.

### Task 2: Make cross-half helpers direction-explicit

**Files:**
- Modify: `src/app/state/selection.rs` (clamping, `active_list_len`, list reset, `sync_track_transition`), `src/app/selection.rs`, `src/app/filter.rs`, `src/app/state/modals.rs` (`modal_capture`)

**Interfaces:**
- Produces: helpers with signatures `fn xyz(domain: &DomainState, ui: &mut UiState, ...)`. Nothing outside `handle_action`/`reduce` takes `&mut` to both halves.

- [ ] **Step 1:** Change each helper to the explicit signature; update call sites.
- [ ] **Step 2:** Full gate; commit `refactor: selection and filter helpers read domain, mutate ui`.

### Task 3: Split the reducer into `reducer::domain` and `reducer::ui`

**Files:**
- Create: `src/app/reducer/ui/mod.rs` (+ `navigation.rs`, `modals.rs`, `presentation.rs`)
- Modify: `src/app/reducer/mod.rs` (router), all existing sub-reducers, `src/app/reducer/tests/**`

**Variant routing (exact):**
- To `reducer::ui::navigation`: `Navigate`, `OpenHelp`, `CloseHelp`, `ScrollHelp`, `NextView`, `PreviousView`, `CycleHomeSection`, `SelectNext`, `SelectPrevious`, `ToggleSearchDetail`, `SearchInput`, `SearchBackspace`, `ClearSearch` (input-buffer part), `BackFromChannel` (view part).
- To `reducer::ui::modals`: all `PlaylistAction` prompt/editor/picker/confirm variants currently in `reducer/playlists/modals.rs`; the `OpenTrackContext`/`CloseTrackContext`/`MoveTrackContext`/`ShowTrackDetails`/`CloseTrackDetails` transitions from `reducer/track_context.rs`.
- To `reducer::ui::presentation`: `ScrollNowPlaying`, `ToggleNowPlayingPane`, `CyclePlayingPane`, `ToggleHistoryViewMode`, `Notify`, `DismissNotification`, `ToggleNotificationLog`, `ClearActivity` (log side).
- Everything else stays in the existing domain sub-reducers, whose signatures change to `pub(super) fn reduce(domain: &mut DomainState, action: X) -> Vec<Effect>`.
- Variants needing both halves (e.g. `SearchCompleted` mutates `search` [D] and resets selection [U]; `SubmitSearch` reads `search_input` [U]): the domain reducer takes only its half; the UI part moves to Task 4's `apply_domain_events` (selection reset on `SearchCompleted`) or into the router which passes the needed value in the action payload (`SubmitSearch` already carries the query — verify; if it reads `state.search_input`, change the keymap/input layer to embed the query in the action at dispatch time).

**Router (top of `reducer/mod.rs`):**

```rust
pub fn reduce(state: &mut AppState, action: Action) -> (Vec<Effect>, Vec<DomainEvent>) // events added in Task 4; Vec<Effect> only until then
```

- [ ] **Step 1:** Move UI-only arms into `reducer/ui/*`, changing their receivers to `&mut UiState` (+ `&DomainState` where clamping needs lengths).
- [ ] **Step 2:** Narrow the domain sub-reducers to `&mut DomainState`; compiler surfaces every remaining cross-half read — resolve each by payload-passing or by deferring the UI half (list kept in the task journal).
- [ ] **Step 3:** Update `reducer/tests/**` construction sites; assertions unchanged.
- [ ] **Step 4:** Full gate; commit `refactor: split reducer into domain and ui halves`.

### Task 4: Introduce the `DomainEvent` seam

**Files:**
- Create: `src/app/domain_event.rs`
- Modify: `src/app/reducer/mod.rs`, `src/app/action_dispatch.rs`, `src/app/runtime.rs`
- Create: `src/app/ui_sync.rs` (`apply_domain_events`)

**Interfaces:**

```rust
/// Broadcast-shaped domain change notifications; Phase 2 sends these over the socket.
#[derive(Debug, Clone, PartialEq)]
pub enum DomainEvent {
    QueueChanged,
    PlaybackChanged,
    TrackChanged,
    TrackDetailsChanged,
    PlaylistsChanged,
    HistoryChanged,
    SearchChanged,
    ChannelChanged,
    ImportChanged,
    Health,
    OperationFailed { message: String },
}

pub fn apply_domain_events(domain: &DomainState, ui: &mut UiState, events: &[DomainEvent]);
```

- [ ] **Step 1:** Domain sub-reducers return `(Vec<Effect>, Vec<DomainEvent>)`; each mutation arm names its event (mapping: queue arms → `QueueChanged`; playback controls/events → `PlaybackChanged`; track load → `TrackChanged`; details → `TrackDetailsChanged`; playlist catalog/imports → `PlaylistsChanged`/`ImportChanged`; history → `HistoryChanged`; search transitions → `SearchChanged`; channel transitions → `ChannelChanged`).
- [ ] **Step 2:** `apply_domain_events` hosts the cross-half reactions deferred from Task 3 (selection reset on `SearchChanged`, clamp on `QueueChanged`/`PlaylistsChanged`, notification on `OperationFailed`), reusing the Task 2 helpers.
- [ ] **Step 3:** `handle_action` collects events from reduce + service handlers and calls `apply_domain_events` once per action, before render.
- [ ] **Step 4:** New unit tests in `src/app/reducer/tests/domain_events.rs`: each mutating action family emits its event; non-mutating actions emit none; `apply_domain_events` clamps selection when the queue shrinks.
- [ ] **Step 5:** Full gate; commit `refactor: domain reducers emit DomainEvent seam`.

### Task 5: Carve services into `src/app/domain/`

**Files:**
- `git mv` into `src/app/domain/`: `background.rs`, `media_tasks.rs`, `playback_followup.rs`, `playback_recovery.rs`, `playback_session.rs`, `persistence.rs`, `operations.rs`, `channel.rs` (+ its tests), `effects.rs`
- Stays UI-side: `thumbnails.rs`, `external_command.rs`, `browser.rs`, `mouse.rs`, `input.rs`, `modal_input.rs`, `filter.rs`, `selection.rs`
- Split: `src/app/service_actions/**` — `playback.rs`, `queue.rs`, `playlists.rs`, `playlist_storage.rs`, `history.rs`, and the search/channel arms of `navigation.rs` become `src/app/domain/services/*` operating on `&mut DomainState` + services and returning `Vec<DomainEvent>`; the modal workflows of `playlist_editing.rs` and the menu parts of `service_actions/track_context.rs` stay UI-side, calling into domain functions for the actual mutations.

- [ ] **Step 1:** Move the pure-domain files; fix module paths only. Gate. Commit `refactor: move domain services under app/domain`.
- [ ] **Step 2:** Split `service_actions` per the table; every domain service function loses access to `UiState` (compiler-enforced) and returns events. Gate. Commit `refactor: service actions split into domain and ui sides`.

### Task 6: Boundary guard + docs

**Files:**
- Create: `src/app/domain/boundary_tests.rs` — walks `src/app/domain/**` and `src/app/state/domain_state.rs` at test time (`std::fs`, no network) and fails if any non-test line contains `ratatui`.
- Modify: `ARCHITECTURE.md` (boundaries section: domain half, UI half, `DomainEvent` seam, Phase 2 pointer), `docs/superpowers/specs/2026-07-21-daemon-split-design.md` status line.

- [ ] **Step 1:** Write the guard test; verify it fails when a `ratatui` import is temporarily added to a domain file, then passes clean.
- [ ] **Step 2:** Update docs. Full gate. Commit `docs: record domain/ui boundary; guard domain against ratatui`.

### Task 7: Final verification

- [ ] Full merge gate; `git diff main` review for behavior drift (grep for changed string literals, changed test expectations — there must be none beyond paths/signatures).
- [ ] Re-run `tests/mpv_ipc.rs` in isolation.
- [ ] Confirm plan checkboxes; note deviations at the bottom of this file.

## Completion notes (2026-07-21)

Tasks 1–4, 5 (first commit), 6, and 7 are implemented; all behavior-change
checks stayed green throughout (existing tests updated for paths/signatures
only). Deviations from the written plan:

- **Task 3** kept per-family coordinators owning cross-half glue instead of
  deferring reactions to Task 4, so every intermediate commit stayed
  behavior-identical. The EOF autoplay path calls the domain `next_track`
  directly rather than recursing through the top-level reducer.
- **Task 4** derives events via `DomainWatermark` (revision/occurrence
  comparison plus counterless action mapping) instead of threading an event
  vector through every reducer signature. Same seam, far less churn;
  `apply_domain_events` reactions are limited to idempotent selection clamps
  because coordinators already apply the specific reactions.
- **Task 5, second commit (service_actions split) deferred to phase 2.**
  The handlers are already thin per-family coordinators; making them
  `UiState`-free requires the daemon runtime struct (natural signatures) and
  moving `ChannelState::return_to` client-side — both phase 2 work. Recorded
  in ARCHITECTURE.md as known debts.
