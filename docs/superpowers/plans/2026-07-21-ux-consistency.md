# UX Consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the 10 approved UX-consistency improvements from the 2026-07-21 design critique: one container grammar, hint-free panel titles, pane gutters, honest time display, one track-table component, one focus grammar, cross-tab state markers, standard empty states, advertised keys, and one vocabulary.

**Architecture:** All changes live in `src/ui` (plus `src/input/keymap.rs` footer/help data and `src/ui/icons.rs`). No reducer/state changes except reading existing fields. Rendering stays pure; tests are ratatui `TestBackend` buffer assertions in module tests and `tests/ui_snapshots/`.

**Tech Stack:** Rust 1.88 (pinned), ratatui, existing `Theme`/`Icons` tokens.

## Global Constraints

- Merge gate (CONTRIBUTING.md): `cargo fmt --all -- --check`, `cargo check --locked --all-targets`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `git diff --check`.
- The UI must work without color and in ASCII icon mode (PRD 20 / 10.12).
- Existing snapshot tests assert rendered strings; update assertions in the same commit as the visual change they cover.
- One commit per task; run the gate before each commit.
- Design language: **chrome** = borderless strips, **pane** = `section_panel` rule-title, **overlay** = rounded border. `section_panel` titles are `NAME (COUNT) · STATE` — never key hints (uppercasing corrupts case-sensitive keys). `--:--` is the only unknown-duration placeholder. Column vocabulary: `TITLE / CHANNEL / LENGTH`.

---

### Task 1: Strip key hints from panel titles; delete the BROWSE MODE row

**Files:**
- Modify: `src/ui/views/history.rs:26` (title → `History ({total}) · {mode}`)
- Modify: `src/ui/views/playing.rs:19-23` (title → `Now Playing`)
- Modify: `src/ui/views/playing_panels.rs:73-77,109-114,152` (`Description`, `Tracklist ({len})`, `Queue`)
- Modify: `src/ui/views/mod.rs:107-115` (help title → `Help`)
- Modify: `src/ui/views/playlists.rs:189-315` (remove BROWSE MODE bottom row + its layout slot)
- Modify: `src/ui/footer.rs` (add `("j/k", "scroll")` to the default NowPlaying hint set so the removed title hint stays discoverable)
- Test: existing snapshot assertions in `tests/ui_snapshots/playing.rs`, `history.rs`, `help.rs` that mention removed title text.

**Steps:** edit titles → `cargo test --locked --all-targets` → fix assertions → gate → commit `fix: remove key hints from panel titles`.

### Task 2: Gutters between panes + named layout constants

**Files:**
- Modify: `src/ui/layout.rs` (add `pub const PANE_GUTTER: u16 = 1;` and split-ratio/threshold constants: `LIST_DETAIL: [u16; 2] = [65, 35]`, `PLAYLIST_MASTER: [u16; 2] = [38, 62]`, `HOME_GRID_MIN_WIDTH: u16 = 68`, `INSPECTOR_MIN_WIDTH: u16 = 88`, `RESUME_ART_MIN_WIDTH: u16 = 36`)
- Modify: `src/ui/views/search.rs`, `channel.rs` (use `LIST_DETAIL` — channel changes 68/32 → 65/35 — and `.spacing(PANE_GUTTER)`)
- Modify: `src/ui/views/playlists.rs`, `home.rs`, `playing.rs`, `playing_hero.rs` (`.spacing(PANE_GUTTER)` on every multi-pane split; replace magic widths 68/88/36)
- Fixes the `00:00Queue ends after this track` collision (Tracklist time column vs Up Next).

**Steps:** edits → gate → commit `fix: add pane gutters and named layout constants`.

### Task 3: Honest time display + Now Playing identity

**Files:**
- Modify: `src/ui/components/playback.rs` `render_status` (show `elapsed / total` with `--:--` fallback, matching the Quick Resume gauge; drop the remaining-time value)
- Modify: `src/ui/views/playing_hero.rs` `render_track_info` (prepend truncated track title in `theme.header` and channel in `theme.accent` above the stats line)
- Test: `src/ui/components/tests` playback assertions; `tests/ui_snapshots/playing.rs`.

**Steps:** edits → gate → commit `fix: show elapsed/total time and title now playing hero`.

### Task 4: One container grammar (chrome / pane / overlay)

**Files:**
- Modify: `src/ui/views/search.rs` `render_input` (bordered block → `section_panel("Search", focused = state.focus == Focus::SearchInput)`; input row inside; outer constraint `Length(3)` → `Length(2)`)
- Modify: `src/ui/widgets.rs` `render_now_playing` (rounded border → single top rule in `theme.border`; content in remaining 3 rows)
- Modify: `src/ui/layout.rs` (`now_playing_height` 5 → 4 + tests)
- Modify: `src/ui/mod.rs` notification (right-aligned, truncated to half the header width, so tabs stay visible)
- Modify: `src/ui/views/playlists.rs` `render_playlist_detail` (name/description band moves inside the panel: title stays `Playlist editor ({count})`, first inner row = name (header style) + description (dim))
- Modify: `src/ui/views/channel.rs:23` (bare fallback paragraph → `section_panel("Channel")` + dim message)

**Steps:** edits → gate (layout tests change: mini-player height 4) → commit `refactor: unify chrome, pane, and overlay container styles`.

### Task 5: One focus grammar

**Files:**
- Modify: `src/ui/components.rs` `section_panel` (marker is always `icons.section_bar`; focused = accent marker + REVERSED accent title; unfocused = dim marker + dim title; `icons.play_btn` no longer used as focus marker — reserved for "playing")
- Modify: `src/ui/views/search.rs` (results pane `focused` = focus not in input)
- Test: `src/ui/components/tests` focus-marker assertions.

**Steps:** edits → gate → commit `fix: one focus treatment, play glyph reserved for playback`.

### Task 6: Shared track-table component + cross-tab state markers + legend

**Files:**
- Create: `src/ui/components/track_table.rs` — produces per-row `Row`s and a shared header with fixed vocabulary (`TITLE / CHANNEL / LENGTH` or caller-supplied right column header e.g. `LISTENED`), `theme.label` header + `bottom_margin(1)`, full-row `theme.selected` highlight, `icons.chevron_r` highlight symbol, ellipsis pre-truncation from the pane width, `--:--` placeholder, and a marker column (playing = green play glyph, queued = cyan queue glyph, in-playlist = orange dot).
- Create: `src/ui/components/track_flags.rs` (or fn in track_table): `track_flags(state, track_id) -> { queued, in_playlist, playing }`.
- Modify: `src/ui/views/search.rs`, `channel.rs`, `playlists.rs` (preview + editor), `queue.rs`, `history.rs` — all six lists render through the component; Channel keeps its `Load more…`/`Retry…` appended rows; History supplies custom right columns.
- Modify: `src/ui/views/search.rs` meta line: append marker legend (`{queue icon} queued · {dot} in playlist`, dim).
- Test: new `track_table` unit tests (truncation, marker column, header) + snapshot updates.

**Steps:** component + tests first, then migrate one view per sub-step running tests between; gate → commit `refactor: shared track table with cross-tab state markers`.

### Task 7: Standard empty-state component

**Files:**
- Create: `empty_state(frame, area, icon, headline, hint_chips: &[(&str, &str)], theme)` in `src/ui/components.rs` — blank line, accent icon + dim headline, hint line as key chips; every empty state names its next key.
- Modify: `queue.rs`, `playlists.rs`, `history.rs`, `search.rs` (idle + no-results), `playing.rs` (nothing playing), `playing_panels.rs` (up next), `channel.rs` (no videos / unavailable, currently unstyled).

**Steps:** helper + migrate → gate → commit `refactor: shared empty-state pattern with action hints`.

### Task 8: Advertise the keys that exist

**Files:**
- Modify: `src/ui/header.rs` `tab_titles` (prefix `1 `–`6 `; hit zones derive automatically; update `widgets.rs` header tests)
- Modify: `src/ui/footer.rs`: fixed slot order (primary → item actions → list actions → global `c` / `/` / `?`); add `("/", "filter")` to Playlists; add `("c", "actions")` to every view where the menu works; drop `q quit` from narrow Search; align narrow/wide Search sets.

**Steps:** edits → gate → commit `fix: numbered tabs and consistent footer hints`.

### Task 9: One vocabulary + small fixes

**Files:**
- Modify: `src/ui/context_menu.rs:81` (`Artist` → `Channel`)
- Modify: `src/ui/views/search_detail.rs:36` (`Unknown` → `--:--`)
- Modify: `src/ui/views/history.rs:74` (map `PlaybackOutcome` variants to lowercase words instead of `{:?}`)
- Modify: `src/ui/header.rs` (tab label `Playing` → `Now Playing`, short `Play`)
- Modify: `src/ui/widgets.rs:13` (`SPINNER_FRAMES` last four glyphs → braille `⠦ ⠧ ⠇ ⠏`)
- Modify: `src/ui/components.rs` (delete unused `button()`)

**Steps:** edits → gate → commit `fix: unified vocabulary, braille spinner, dead code removal`.

### Task 10: Final verification

- Full merge gate; review `git diff main` for stray debug/scope creep; confirm all snapshot suites pass in ASCII and truecolor paths.
