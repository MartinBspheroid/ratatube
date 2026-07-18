# Track Context Menu, Channel Browser, and End-of-Track Transition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a universal `c` track context menu, a newest-first paginated channel browser, and a one-shot final-15-seconds current-to-next title transition.

**Architecture:** Resolve selected tracks and allowed actions through one view-independent context boundary. Channel metadata and pages flow through cancellable yt-dlp operations into a dedicated non-tab view. The shared playback component renders a state-driven transition using the queue's effective next track.

**Tech Stack:** Rust 2024, Ratatui, Tokio, yt-dlp subprocesses, Serde, Crossterm, existing operation registry and queue/playlist services.

## Global Constraints

- Playback continues while the context menu is open.
- Channel videos are newest first and load in bounded pages through an explicit `Load more...` row.
- No clipboard dependency is added; platform commands receive validated URLs through stdin without a shell.
- Existing persisted tracks remain compatible through optional channel fields with Serde defaults.
- Remote text is sanitized before rendering and narrow layouts remain usable.
- No file may exceed 250 lines as a result of this work; add focused modules instead of expanding `src/app/mod.rs` or `src/ui/mod.rs` with feature logic.
- Every exported function and type receives a documentation comment.
- Every task follows test-first development and runs the narrowest relevant gate before commit.
- The worktree is already dirty. Before task commits, establish an isolated worktree from the approved spec commit or commit the current baseline; never stage unrelated existing changes.

---

### Task 1: Persist Optional Channel Identity

**Files:**
- Modify: `src/media/mod.rs`
- Modify: `src/media/yt_dlp.rs`
- Modify: `src/playlists/model.rs`
- Test: `src/media/yt_dlp.rs`
- Test: `src/playlists/model.rs`
- Test: `tests/yt_dlp.rs`

**Interfaces:**
- Produces: `Track.channel_id: Option<String>` and `Track.channel_url: Option<String>`.
- Produces: yt-dlp normalization that fills both fields from `channel_id` and `channel_url`.

- [ ] **Step 1: Write failing compatibility and parser tests**

Add a parser case with `channel_id` and `channel_url`, and a persisted-track case without either field:

```rust
#[test]
fn parses_stable_channel_identity() {
    let entry: YtDlpEntry = serde_json::from_str(
        r#"{"id":"v","title":"Video","channel":"Channel","channel_id":"UC123","channel_url":"https://www.youtube.com/channel/UC123"}"#,
    ).expect("entry");
    let track = entry.into_track().expect("track");
    assert_eq!(track.channel_id.as_deref(), Some("UC123"));
    assert_eq!(track.channel_url.as_deref(), Some("https://www.youtube.com/channel/UC123"));
}

#[test]
fn legacy_track_defaults_channel_identity() {
    let track: Track = serde_json::from_str(
        r#"{"id":"v","title":"Video","artist":"Channel","webpageUrl":"https://www.youtube.com/watch?v=v","durationSeconds":null,"thumbnailUrl":null,"availability":"unknown"}"#,
    ).expect("legacy track");
    assert_eq!(track.channel_id, None);
    assert_eq!(track.channel_url, None);
}
```

- [ ] **Step 2: Run the narrow tests and verify failure**

Run: `cargo test --locked channel_identity`

Expected: compilation fails because `Track` and `YtDlpEntry` have no channel identity fields.

- [ ] **Step 3: Add optional fields and propagate them through playlist conversion**

Use this model shape:

```rust
#[serde(default)]
pub channel_id: Option<String>,
#[serde(default)]
pub channel_url: Option<String>,
```

Initialize both to `None` in `Track::new`, parse them in `YtDlpEntry`, copy them in `into_track`, and preserve them in both `From<&Track> for PlaylistTrack` and `From<&PlaylistTrack> for Track`.

- [ ] **Step 4: Verify Task 1**

Run: `cargo test --locked channel_identity && cargo test --locked playlist_roundtrips && cargo test --locked --test yt_dlp`

Expected: channel parser and legacy persistence tests pass with no playlist round-trip regression.

- [ ] **Step 5: Commit only Task 1 files**

```bash
git add src/media/mod.rs src/media/yt_dlp.rs src/playlists/model.rs tests/yt_dlp.rs
git commit -m "feat: preserve track channel identity"
```

### Task 2: Build the Universal Track Context Resolver

**Files:**
- Create: `src/app/track_context.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/app/state.rs`
- Modify: `src/app/action.rs`
- Test: `src/app/track_context.rs`

**Interfaces:**
- Produces: `TrackSource`, `TrackContextAction`, `TrackContext`, and `resolve_track_context(&AppState, Option<&HistoryService>) -> Option<TrackContext>`.
- Produces: `AppState.track_context_menu: Option<TrackContextMenuState>`.
- Consumes: optional channel fields from Task 1.

- [ ] **Step 1: Write table-driven resolver tests**

Cover Search, Queue, Playlist detail, History Recent, History Top, a populated Channel state, Playing, and Home Recent. Assert exact source identity and action ordering. Include duplicate queue and playlist tracks to prove the resolver stores an occurrence index rather than only a video ID.

```rust
assert_eq!(
    context.actions,
    vec![
        TrackContextAction::PlayNow,
        TrackContextAction::PlayNext,
        TrackContextAction::AddToPlaylist,
        TrackContextAction::VisitChannel,
        TrackContextAction::ShowDetails,
        TrackContextAction::OpenInBrowser,
        TrackContextAction::CopyUrl,
        TrackContextAction::RemoveFromQueue { order_index: 1 },
    ]
);
```

- [ ] **Step 2: Run resolver tests and verify failure**

Run: `cargo test --locked app::track_context::tests`

Expected: compilation fails because the resolver module and types do not exist.

- [ ] **Step 3: Implement the focused context module**

Define stable action data rather than closures:

```rust
pub enum TrackSource {
    Search,
    Queue { order_index: usize },
    Playlist { playlist_id: String, track_index: usize },
    History,
    Channel,
    Playing,
    Home,
}

pub struct TrackContext {
    pub track: Track,
    pub source: TrackSource,
    pub actions: Vec<TrackContextAction>,
}
```

Hide redundant add-to-queue actions when the selected video already exists in the queue. Include playlist removal only when the resolver has both playlist ID and exact track index.

- [ ] **Step 4: Add modal state and open/close/select actions**

Add `OpenTrackContext`, `CloseTrackContext`, `MoveTrackContext(i32)`, and `SubmitTrackContext`. Opening with no resolved track must call `state.notify("No track selected", true)` and leave the modal absent.

- [ ] **Step 5: Verify Task 2**

Run: `cargo test --locked app::track_context && cargo test --locked track_context`

Expected: all source/action matrices and occurrence-identity cases pass.

- [ ] **Step 6: Commit only Task 2 files**

```bash
git add src/app/track_context.rs src/app/mod.rs src/app/state.rs src/app/action.rs
git commit -m "feat: resolve universal track context actions"
```

### Task 3: Render and Operate the Non-Blocking Context Menu

**Files:**
- Create: `src/ui/context_menu.rs`
- Create: `src/platform/clipboard.rs`
- Create: `src/platform/mod.rs`
- Modify: `src/lib.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/input/keymap.rs`
- Modify: `src/app/mod.rs`
- Test: `src/platform/clipboard.rs`
- Test: `tests/ui_snapshots.rs`

**Interfaces:**
- Consumes: `TrackContextMenuState` and `TrackContextAction` from Task 2.
- Produces: `copy_url(url: &str) -> Result<()>` and context action dispatch.

- [ ] **Step 1: Write failing modal and input tests**

Assert `c` opens the menu in each track-bearing view, `j/k` changes only menu selection, `Esc` closes it, and playback status remains `Playing`. Snapshot the exact action order and warning style marker for removal actions.

- [ ] **Step 2: Write failing clipboard adapter tests**

Use a temporary fake executable that records stdin. Assert a valid YouTube URL reaches stdin byte-for-byte, malformed or non-HTTPS URLs are rejected before spawning, and a non-zero command exit returns an error.

- [ ] **Step 3: Run tests and verify failure**

Run: `cargo test --locked track_context_menu && cargo test --locked clipboard`

Expected: missing renderer, key binding, and clipboard adapter failures.

- [ ] **Step 4: Implement the menu renderer and modal-first input routing**

Render the modal after main content and before notifications. Use `Clear`, a bounded centered rectangle, sanitized title, `List`, and warning styling for removal actions. Route input before global keys:

```rust
if self.state.track_context_menu.is_some() {
    let action = match key.code {
        KeyCode::Esc => Some(Action::CloseTrackContext),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::MoveTrackContext(1)),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::MoveTrackContext(-1)),
        KeyCode::Enter => Some(Action::SubmitTrackContext),
        _ => None,
    };
    if let Some(action) = action {
        let _ = action_tx.send(action).await;
    }
    return;
}
```

- [ ] **Step 5: Implement browser, clipboard, details, queue, and playlist dispatch**

Keep browser and clipboard failures inside the open menu. Reuse existing queue and playlist actions. For removals, dispatch occurrence-bearing actions and validate the target still matches before mutation.

- [ ] **Step 6: Verify Task 3**

Run: `cargo test --locked clipboard && cargo test --locked context_menu && cargo test --locked --test ui_snapshots`

Expected: context menu works on every supported surface and playback state never changes merely from opening it.

- [ ] **Step 7: Commit only Task 3 files**

```bash
git add src/ui/context_menu.rs src/platform/clipboard.rs src/platform/mod.rs src/lib.rs src/ui/mod.rs src/input/keymap.rs src/app/mod.rs tests/ui_snapshots.rs
git commit -m "feat: add non-blocking universal track menu"
```

### Task 4: Add Bounded Channel Metadata and Page Fetching

**Files:**
- Create: `src/media/channel.rs`
- Modify: `src/media/mod.rs`
- Modify: `src/media/yt_dlp.rs`
- Test: `src/media/channel.rs`
- Test: `tests/yt_dlp.rs`

**Interfaces:**
- Produces: `ChannelPageRequest`, `ChannelPage`, and `YtDlp::fetch_channel_page(&ChannelPageRequest) -> Result<ChannelPage>`.
- Consumes: `Track.channel_url` from Task 1.

- [ ] **Step 1: Write failing page-boundary and normalization tests**

Use page size 30. Page 0 must invoke `--playlist-start 1 --playlist-end 30`; page 1 must invoke `--playlist-start 31 --playlist-end 60`. Verify newest-first input order is retained, malformed/private rows are counted, duplicate IDs are removed, and fewer than 30 accepted source entries marks exhaustion.

- [ ] **Step 2: Run tests and verify failure**

Run: `cargo test --locked channel_page && cargo test --locked --test yt_dlp channel`

Expected: missing channel fetch types and method.

- [ ] **Step 3: Implement channel URL normalization and page types**

```rust
pub const CHANNEL_PAGE_SIZE: usize = 30;

pub struct ChannelPageRequest {
    pub channel_url: String,
    pub page: usize,
}

pub struct ChannelPage {
    pub tracks: Vec<Track>,
    pub rejections: ImportRejections,
    pub exhausted: bool,
}
```

Accept only HTTPS `youtube.com` and `www.youtube.com` channel URLs. Normalize the fetch target to a single trailing `/videos` and reject query strings, fragments, lookalike hosts, and video URLs.

- [ ] **Step 4: Implement bounded direct-argument yt-dlp execution**

Use `--dump-json --flat-playlist --no-download --ignore-errors --playlist-start N --playlist-end M -- URL`. Parse one JSON object per line so partial valid rows survive malformed output. Determine exhaustion from the number of source lines returned, not accepted tracks, so private rows do not terminate pagination early.

- [ ] **Step 5: Verify Task 4**

Run: `cargo test --locked media::channel && cargo test --locked --test yt_dlp`

Expected: exact argument, URL safety, partial-result, rejection-count, and exhaustion tests pass.

- [ ] **Step 6: Commit only Task 4 files**

```bash
git add src/media/channel.rs src/media/mod.rs src/media/yt_dlp.rs tests/yt_dlp.rs
git commit -m "feat: fetch bounded channel video pages"
```

### Task 5: Add the Dedicated Channel View and Cancellable Data Flow

**Files:**
- Create: `src/app/channel.rs`
- Create: `src/ui/views/channel.rs`
- Modify: `src/app/action.rs`
- Modify: `src/app/operations.rs`
- Modify: `src/app/state.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/ui/views/mod.rs`
- Modify: `src/ui/widgets.rs`
- Test: `src/app/channel.rs`
- Test: `tests/ui_snapshots.rs`

**Interfaces:**
- Consumes: channel fetch types from Task 4 and context menu from Tasks 2-3.
- Produces: `View::Channel`, `ChannelState`, metadata/page completion actions, retry/load-more/back behavior.

- [ ] **Step 1: Write failing reducer and stale-operation tests**

Assert opening a channel snapshots prior view/focus/selection, initial success appends unique rows, a stale completion is ignored, load-more cannot double-submit, later failure preserves rows, retry retains the same page, and Back restores the snapshot.

- [ ] **Step 2: Write failing responsive UI tests**

Snapshot Loading, initial failure, empty, populated, retry-load-more, and exhausted states at 80, 120, 150, and 180 columns. Assert table-only narrow output and table-plus-preview wide output. Assert `Load more...` is selectable but never treated as a track context.

- [ ] **Step 3: Run tests and verify failure**

Run: `cargo test --locked app::channel && cargo test --locked --test ui_snapshots channel_view`

Expected: missing Channel view, state, actions, and renderer.

- [ ] **Step 4: Implement isolated channel state and operation kinds**

Add `ChannelResolve` and `ChannelPage` to `OperationKind`. Put channel lifecycle helpers in `src/app/channel.rs`, including append-time ID deduplication with `HashSet<String>`. Store the prior navigation snapshot in channel state and do not add Channel to `View::TABS`.

- [ ] **Step 5: Wire Visit Channel and page effects**

When channel identity is missing, fetch full video metadata first. Only navigate after a safe channel URL exists. Spawn page requests through `OperationRegistry`, carry `OperationId` in completion actions, and reject completions that do not match the active channel and operation.

- [ ] **Step 6: Render the Channel view and actions**

Use Ratatui `Table` with title, channel, and duration. The final synthetic row dispatches Load More or Retry. Real rows support Play, Play Next, Add to Queue, Add to Playlist, and `c`.

- [ ] **Step 7: Verify Task 5**

Run: `cargo test --locked app::channel && cargo test --locked channel_view && cargo test --locked track_context`

Expected: navigation restoration, cancellation, pagination, context actions, and every responsive state pass.

- [ ] **Step 8: Commit only Task 5 files**

```bash
git add src/app/channel.rs src/ui/views/channel.rs src/app/action.rs src/app/operations.rs src/app/state.rs src/app/mod.rs src/ui/views/mod.rs src/ui/widgets.rs tests/ui_snapshots.rs
git commit -m "feat: add paginated channel browser"
```

### Task 6: Model the Effective Next Track and One-Shot Transition

**Files:**
- Create: `src/playback/transition.rs`
- Modify: `src/playback/mod.rs`
- Modify: `src/queue/model.rs`
- Modify: `src/app/state.rs`
- Modify: `src/app/mod.rs`
- Test: `src/playback/transition.rs`
- Test: `src/queue/model.rs`

**Interfaces:**
- Produces: `Queue::effective_next(&self) -> Option<&Track>` without mutation.
- Produces: `TrackTransitionState::update(TransitionInput, Instant)` and `TrackTransitionState::progress(Instant) -> Option<f64>`.

- [ ] **Step 1: Write failing effective-next tests**

Cover linear queue, final item with repeat off, queue wrap, repeat track suppression, and shuffled order. Calling `effective_next` must not mutate queue position, order, or play history.

- [ ] **Step 2: Write failing transition state-machine tests**

Use controlled `Instant` values. Assert crossing 15 seconds arms once, pause freezes elapsed progress, resume continues it, seeking above 15 rearms it, seeking within the threshold does not restart it, and changing track resets it.

- [ ] **Step 3: Run tests and verify failure**

Run: `cargo test --locked effective_next && cargo test --locked playback::transition`

Expected: missing queue query and transition module.

- [ ] **Step 4: Implement the non-mutating queue query**

Resolve the next order position from current `position`. Return `None` for repeat-track and a terminal repeat-off queue. Wrap only for repeat-queue. Shuffle already lives in `order`, so do not randomize or advance inside this method.

- [ ] **Step 5: Implement the state machine**

```rust
pub struct TransitionInput<'a> {
    pub track_id: Option<&'a str>,
    pub remaining_seconds: Option<f64>,
    pub playing: bool,
    pub has_next: bool,
}
```

Store active track ID, prior remaining time, armed/started/completed state, accumulated active duration, and last resume instant. Use a 15-second threshold and a fixed animation duration constant. No renderer mutation is permitted.

- [ ] **Step 6: Feed playback events into transition state**

After playback snapshot and queue state update, call the transition state with the same `Instant` for deterministic edge detection. Reset on current-track replacement and queue exhaustion.

- [ ] **Step 7: Verify Task 6**

Run: `cargo test --locked queue::model && cargo test --locked playback::transition`

Expected: all modes, threshold edges, pause/resume, seek, and reset cases pass.

- [ ] **Step 8: Commit only Task 6 files**

```bash
git add src/playback/transition.rs src/playback/mod.rs src/queue/model.rs src/app/state.rs src/app/mod.rs
git commit -m "feat: model final-track transition state"
```

### Task 7: Render the Current-to-Next Playback Transition

**Files:**
- Modify: `src/ui/components.rs`
- Modify: `src/ui/icons.rs`
- Test: `src/ui/components.rs`
- Test: `tests/ui_snapshots.rs`

**Interfaces:**
- Consumes: `Queue::effective_next` and `TrackTransitionState` from Task 6.
- Produces: width-safe styled transition title in the shared bottom playback component.

- [ ] **Step 1: Write failing component and snapshot tests**

Assert normal title before threshold; current title in cyan, configured left chevron, and next title in base white during transition; one-cell-safe clipping at 60 and 80 columns; ASCII `<`; Nerd Font `‹`; no transition without a next track or under repeat-track.

- [ ] **Step 2: Run tests and verify failure**

Run: `cargo test --locked playback_transition && cargo test --locked components::tests`

Expected: shared component still renders only the normal title.

- [ ] **Step 3: Implement styled sliding-line composition**

Create a private helper returning `Line<'static>` from sanitized current/next titles, available width, and normalized progress. Compute a leading offset from progress, clip by terminal display width rather than bytes, and preserve styles across clipped spans. Once progress reaches 1.0, render the final readable `current ‹ next` composition without movement.

- [ ] **Step 4: Verify Task 7**

Run: `cargo test --locked ui::components && cargo test --locked --test ui_snapshots`

Expected: animation frames, final resting state, icon modes, Unicode, and narrow widths pass.

- [ ] **Step 5: Commit only Task 7 files**

```bash
git add src/ui/components.rs src/ui/icons.rs tests/ui_snapshots.rs
git commit -m "feat: render one-shot next-track transition"
```

### Task 8: Documentation and Full Production Gate

**Files:**
- Modify: `README.md`
- Modify: `PRD.md`
- Modify: `ARCHITECTURE.md`
- Modify: `dependencies.md` only if implementation introduced a dependency; the approved design expects none.

**Interfaces:**
- Consumes: completed behavior from Tasks 1-7.
- Produces: discoverable key/action and architecture documentation.

- [ ] **Step 1: Document user behavior and boundaries**

Document `c`, menu keys, contextual removals, Channel Back/Load More/Retry, newest-first ordering, clipboard tool requirements, and the final-15-seconds transition. Document channel operation ownership and stale-result rejection in `ARCHITECTURE.md`.

- [ ] **Step 2: Verify commands and names against the implementation**

Run: `rg -n "TrackContext|ChannelPage|effective_next|TrackTransition|OpenTrackContext" src README.md PRD.md ARCHITECTURE.md`

Expected: documentation names and shortcuts match real symbols and key bindings; no obsolete shortcut is advertised.

- [ ] **Step 3: Run the complete gate**

```bash
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

Expected: every command exits 0. Record live-network tests that remain intentionally ignored; do not present them as executed.

- [ ] **Step 4: Review the final diff for scope and generated files**

Run: `git status --short && git diff --stat && git diff --check`

Expected: only feature, test, and documentation files are changed; no `target`, generated media, secrets, or unrelated dirty-worktree files are staged.

- [ ] **Step 5: Commit documentation and final integration changes**

```bash
git add README.md PRD.md ARCHITECTURE.md dependencies.md
git commit -m "docs: document track context and channel browsing"
```
