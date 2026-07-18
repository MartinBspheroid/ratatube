# Track Context Menu, Channel Browser, and End-of-Track Transition

## Objective

Add a universal track context menu, a first-class channel-video browser, and a one-shot next-track transition in the shared playback component. Playback must continue while menus and channel browsing are active.

## Track Context Menu

Pressing `c` on any surface with a resolvable track opens a centered modal. The modal captures navigation input but does not pause playback. `j` and `k` move the selection, `Enter` executes the selected action, and `Esc` closes the menu.

A central track-context resolver maps the active view and visible selection to:

- The selected `Track`.
- Its source surface.
- Its queue or playlist location when applicable.
- The actions valid for that source.

The menu renders resolved actions but does not mutate application state directly. Each choice dispatches a normal application action.

Actions appear in this order when applicable:

1. Play now.
2. Play next.
3. Add to queue.
4. Add to playlist.
5. Visit channel.
6. Show details.
7. Open video in browser.
8. Copy video URL.
9. Remove from queue.
10. Remove from playlist.

Redundant actions are hidden rather than disabled. Removal actions appear last and use the warning style. Remove-from-playlist is available from both the selected-track preview and full playlist editor when a concrete track is selected.

## Channel Browser

Visiting a channel opens a dedicated `View::Channel`. It is not a primary header tab. `Esc` restores the exact prior view, focus, and selection.

The view contains:

- The channel name and loading status.
- A Ratatui table of videos ordered newest first.
- A selected-video thumbnail and compact metadata preview where width permits.
- A final `Load more...` row while additional results may exist.

The first bounded page loads when the view opens. Selecting `Load more...` fetches the next bounded page and appends results. A request cannot be submitted twice while loading. Existing rows remain usable after a pagination failure, and the final row becomes `Retry load more...`. The same universal context menu and existing fast track actions work on channel rows.

## Channel Metadata and Fetching

`Track` and full yt-dlp metadata gain optional stable `channel_id` and `channel_url` fields. Existing persisted data remains compatible because both fields are optional.

When a selected track already has a trusted channel URL, the app opens it directly. Otherwise it fetches full metadata for the track's existing video URL, extracts the channel identity, and then opens the channel. A missing channel URL produces a visible error and leaves the current view intact.

Channel requests target the channel's `/videos` surface through direct yt-dlp argument execution. Requests are bounded using explicit page boundaries. Pages are normalized through the existing track parser. Duplicate video IDs, including overlap at page boundaries, are discarded while preserving newest-first order. Deleted, private, unavailable, and malformed entries are omitted and counted.

Channel state stores:

- Channel name, ID, and canonical URL.
- Loaded videos.
- Next page boundary.
- Loading, error, and exhausted status.
- Rejection counts.
- Previous view, focus, and selection snapshot.

Channel metadata resolution and page loading use the existing operation registry. Opening another channel cancels previous channel work, stale results are rejected, and leaving the view prevents late completion from changing visible state.

## Clipboard and Browser Boundaries

Browser opening reuses the existing validated URL boundary. Clipboard copying uses a small platform command adapter: `pbcopy` on macOS, `wl-copy` or `xclip` on Linux, and `clip` on Windows. Commands receive the URL through standard input and never through a shell. Success is reported only after a zero exit status. Missing tools, invalid URLs, timeouts, and write failures produce visible errors.

No clipboard dependency is added.

## End-of-Track Transition

The shared bottom playback component owns the transition presentation. It activates once per current track when:

- Duration is known.
- Playback is active.
- Remaining time crosses from above 15 seconds to 15 seconds or less.
- The queue can resolve a concrete effective next track.

The title row changes to a right-to-left transition containing the current title in cyan, the left-chevron icon, and the next title in white. The animation runs once, then remains in a readable final position until the track changes. It never loops.

Pausing freezes animation progress and resuming continues it. Seeking above the threshold rearms the transition. Seeking within the final 15 seconds does not restart it. A track change resets all transition state. Repeat-track mode suppresses the transition. Queue repeat and shuffle use the queue model's actual effective next track. If no next track exists, the regular title remains.

Animation state belongs to application state rather than renderer side effects. Playback events update threshold and timing state; rendering derives the visible offset from that state and elapsed active playback time.

## Responsive and Accessibility Behavior

The context menu width is bounded to the terminal and long titles are safely truncated. The Channel view degrades from table plus preview to table only on narrow terminals. All user and remote text is sanitized before rendering. ASCII icon mode uses `<`; Nerd Font mode uses the configured single-cell left chevron.

The menu exposes explicit action labels and does not depend on color alone. Loading, retry, empty, and exhausted states have textual labels.

## Failure Handling

- No selected track: `c` does nothing except show `No track selected`.
- Channel metadata unavailable: preserve the current view and report the failure.
- Initial channel load fails: show an error with Retry and Back actions.
- Later page fails: preserve loaded videos and offer Retry.
- Clipboard or browser command fails: keep the menu open and report the exact operation failure.
- A removal target disappears before confirmation: cancel safely and refresh selection.
- Empty channel: show `No public videos found` without offering Load more.

## Verification

Unit tests cover context resolution and action capabilities for every track-bearing view, queue and playlist occurrence identity, channel URL normalization, pagination boundaries, page deduplication, stale-operation rejection, effective-next-track resolution, and transition threshold/rearming behavior.

Integration tests use a fake yt-dlp executable to verify channel metadata parsing, bounded page arguments, malformed output, partial results, timeout behavior, and pagination exhaustion. Clipboard command tests use fake executables and verify stdin input, exit status handling, and absence of shell interpolation.

UI tests cover context-menu ordering and conditional actions, playback continuing behind the modal, Channel loading/empty/error/retry/exhausted states, Back restoration, responsive channel layouts, Unicode sanitization, and transition rendering at narrow and wide widths.

The completion gate is:

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

## Acceptance Criteria

- `c` opens the same contextual track-action menu anywhere a track is selected.
- The context menu does not pause or interrupt playback.
- Users can open a channel, browse newest-first videos, incrementally load more, and apply normal track actions.
- Channel failures are recoverable without losing already loaded rows or the previous view.
- URL copying and browser opening are validated and truthfully reported.
- During the final 15 seconds, the current-to-next title transition runs exactly once when a real next track exists.
- Existing persisted tracks and playlists load without migration failures.
- All completion-gate commands pass.
