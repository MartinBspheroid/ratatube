# Design Language

The visual and interaction rules every view, overlay, and component follows.
`ARCHITECTURE.md` says how the code is structured; this file says how the app
looks, speaks, and responds. When adding a feature, match these rules; when a
rule must change, change it here in the same commit.

## Principles

1. **Keyboard-first, mouse-equal.** Every mouse gesture maps onto an existing
   keyboard action — a double-click synthesizes Enter through the normal key
   path (`src/app/mouse.rs`), never a parallel code path. New interactions
   must have a keyboard binding first.
2. **Chrome, not boxes.** Full borders are reserved for overlays/modals.
   Views separate panels with horizontal rules (`panel_rule`), section bars
   (`section_bar` `▎`), and spacing — never nested bordered blocks.
3. **Degrade explicitly.** Everything must work at 80x24, in ASCII, and
   without truecolor. Enhanced glyphs and RGB color are progressive
   enhancements, never requirements (PRD 20).
4. **State is visible.** Loading, cancellation, failure, and non-durable
   state always render; nothing fails silently (PRD 16–18).

## Color and themes

- All color flows through the `Theme` struct (`src/ui/theme.rs`) — 25 named
  style roles (`accent`, `dim`, `selected`, `panel_title`, `key_chip`, …).
  **Never construct a `Style` with literal colors in a view or component**;
  add or reuse a role instead.
- Themes are defined as `Palette`s (`src/ui/theme/palettes.rs`): 13 color
  slots (`bg`, `panel_bg`, `text`/`subtext`/`dim` foreground tiers, `accent`,
  `accent_alt`, semantic `red`/`yellow`/`orange`/`green`, `selected_bg`,
  `border`) mapped to roles by one shared `from_palette` builder. A new theme
  is one `Palette` const plus a `VARIANTS` row — no per-theme styling logic.
- 16 families x dark/light = 32 variants (`src/config/theme.rs`,
  `VARIANTS` is the single source of truth). Every family ships both modes.
  Palette values are the scheme's official published colors; cite the source
  role in a doc comment when the mapping isn't obvious.
- Non-truecolor terminals collapse to the ANSI fallback; never emit RGB
  without checking `theme.truecolor`.
- `accent` carries identity (selection, active tab, gauge); `accent_alt` is
  its partner for gradients/secondary emphasis (e.g. the level-meter lerp).
  Semantic colors keep their meaning: red = error/destructive,
  yellow = warning, green = success/playing.

## Typography and copy

- Panel titles and table headers are UPPERCASE ("NOW PLAYING", "TITLE",
  "LENGTH", "CHANNEL"). Body copy is sentence case. Key hints are lowercase
  chips (`key_chip` role).
- Copy is terse and stable: snapshot tests in `tests/ui_snapshots/` pin
  rendered strings verbatim, so any copy change requires a same-commit sweep
  of those assertions. Treat wording as API.
- Foreground has three tiers: `text` (values, headers, selected),
  `subtext` (secondary copy, chips), `dim` (labels, rules, hints). Pick the
  tier by importance, not by taste.
- Truncation is always trailing-ellipsis via the shared helpers in
  `src/ui/widgets.rs`; never let text overflow or wrap in list rows.

## Iconography

- All glyphs come from the semantic `Icons` slots (`src/ui/icons.rs`) with
  Nerd Font and plain-text variants per slot. Never inline a glyph literal in
  a view — add a slot so ASCII mode stays complete.
- Animation vocabulary: braille spinner (`SPINNER_FRAMES`) for indeterminate
  work, `spectrum_ramp` bar glyphs for audio levels. Motion appears only in
  the playback bar and loading states; static views stay static.

## Layout

- Responsive breakpoints (`src/ui/layout.rs`): Narrow < 100 cols,
  Medium 100–139, Wide 140–169, UltraWide ≥ 170. Extra panes (e.g. the
  Playing queue pane) appear at wider breakpoints; features never become
  unreachable at Narrow — they relocate or stack.
- The bottom playback bar is a four-row chrome strip: rule, title,
  full-width timeline, status row. The status row is measured, not
  fixed-column: time left, controls right, optional 6-band level meter
  centered in the true gap (skipped when the gap is too small).
- Overlays are centered modals with full borders; settings modal width is a
  named constant (`MODAL_WIDTH`). Modals capture input while open but never
  pause playback.
- Magic geometry gets a named constant with a doc comment
  (`HOME_GRID_MIN_WIDTH`, `INSPECTOR_MIN_WIDTH`, …), never an inline number.

## Interaction

- Views register hit geometry during render (`list_hit_area`,
  `list_hit_offset`, `home_hit_zones`); mouse handling never recomputes
  layout. Any manually-windowed list must publish its window offset.
- Single click selects (and focuses the pane/section); double-click within
  500 ms on the same view+pane+item acts as Enter. Clicks never fire actions
  a keypress couldn't.
- Scroll wheel moves selection by 3; on Now Playing it scrolls the view.
- Destructive bulk actions require confirmation; single queue-item deletion
  gets one-level undo. Errors notify with longer visibility than info.
