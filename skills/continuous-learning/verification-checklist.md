# Verification Learning Record

## Context

The adversarial remediation changed persistence, async orchestration, IPC, UI behavior, CLI contracts, documentation, and dependencies over ten short iterations.

## Symptom

Two focused-test attempts passed several filter names as separate positional arguments to `cargo test`; Cargo accepts only one test-name filter. A later UI test also assumed queue storage order was presentation order, even though `Queue::order` is the public play-order contract.

## Root cause (rule)

Verification commands and assertions must follow the tool/domain contract, not a convenient mental model: Cargo has one filter slot, and queue behavior is observed through play order rather than backing-vector position.

## Fix

Use one filter per invocation or run the whole target (`cargo test --lib`, `cargo test --test NAME`). Assert queue behavior by mapping `Queue::order` through `Queue::tracks`.

## Prevention check

- Before running a focused command, verify its positional-argument grammar with `--help` when more than one filter is planned.
- Prefer target-level suites after a red/green focused cycle.
- In tests, assert through the same accessor/index mapping consumed by production code.
- End every multi-iteration task with the complete documented gate and `git diff --check`.

## Learning: Prefer relative commands for relative IPC controls

- **Context:** Repeated volume keys are queued to mpv while observed property-change events update the same local snapshot asynchronously.
- **Symptom:** `-` reached 0%, but every later `+` sent an absolute 2% and volume never rose further.
- **Root cause:** A relative user intent must not be converted to an absolute command from a snapshot that can be overwritten by a stale asynchronous event.
- **Fix:** Send mpv `["add", "volume", delta]`, cap mpv with `--volume-max=100`, and retain the optimistic snapshot only for immediate UI feedback.
- **Prevention check:** For every queued increment/decrement control, inject an older observed event between two commands and verify both command payloads remain relative; then verify the sequence against the real adapter.
- **Tags:** ipc, async, race-condition, playback, testing

## Learning: Convert screenshot annotations into absence and placement tests

- **Context:** A TUI cleanup was specified by red annotations around duplicated status, transport, and meter elements, followed by a pane-order correction.
- **Symptom:** Removing broad status groups could also remove adjacent information the annotation intentionally left untouched, while plain-text snapshots could not prove that tab padding shared the active background.
- **Root cause:** Visual annotations define element-level boundaries and style behavior that text-presence assertions alone do not fully encode.
- **Fix:** Map each marked region to a named UI fragment, add explicit absence assertions for removed fragments, an ordering assertion for swapped panes, and buffer-style assertions for highlighted padding cells.
- **Prevention check:** Before editing from an annotated screenshot, list every enclosed element and every adjacent element that must remain; verify content, ordering, and cell styles separately.
- **Tags:** tui, visual-regression, screenshots, testing

## Learning: Separate persisted field names from UI semantics

- **Context:** YouTube `uploader`/`channel` metadata is retained in the legacy persisted `artist` field for backward compatibility.
- **Symptom:** Tables labeled a channel handle as `ARTIST`, and compact rows placed the channel before the video title.
- **Root cause:** A storage field name is not a user-facing semantic contract; renderers must use the meaning of the upstream value, not its legacy identifier.
- **Fix:** Label the value `CHANNEL` in every UI table and render compact video rows as `{title} — {channel}` while leaving persisted schemas unchanged.
- **Prevention check:** For metadata aliases, test both the visible column label and field order with distinct values; search all renderers for direct use of the legacy field before completing the change.
- **Tags:** metadata, persistence, tui, content-integrity, compatibility

## Learning: Treat terminal paste as a payload event

- **Context:** A TUI field needed to accept pretty-printed, multiline playlist JSON.
- **Symptom:** Character-only prompt handling makes embedded newlines indistinguishable from Enter-to-submit and can lose invalid input after validation.
- **Root cause:** Structured clipboard data is one payload, not a sequence of command keystrokes.
- **Fix:** Enable bracketed paste, handle `Event::Paste` as one bounded action, validate the complete document before writes, and retain rejected text for correction.
- **Prevention check:** Paste a multiline payload containing newlines, verify one submit imports it, verify malformed input remains editable, and verify multi-record persistence rolls back newly written files on failure.
- **Tags:** tui, clipboard, json, validation, persistence

## Learning: Budget popup content rows explicitly

- **Context:** A Ratatui metadata popup contained two bordered three-row fields, spacing, instructions, and outer margins.
- **Symptom:** The description value disappeared even though state and rendering code were correct.
- **Root cause:** Fixed constraints plus inter-row spacing exceeded the popup's inner height, so layout compression removed the field's content row.
- **Fix:** Calculate the full vertical budget including borders, margins, and spacing, then size the popup to preserve every input row.
- **Prevention check:** For each popup breakpoint, assert that every field label and a distinctive field value are both rendered; sum constraints, spacing, margins, and borders before choosing modal height.
- **Tags:** ratatui, layout, popup, forms, visual-regression

## Learning: Probe alternate YouTube channel surfaces

- **Context:** Channel browsing normalized every YouTube channel URL to its `/videos` tab.
- **Symptom:** Valid `- Topic` channels opened the Channel view but failed with yt-dlp's “does not have a videos tab” error.
- **Root cause:** A valid channel identity does not guarantee that every channel type exposes the same tab surface; YouTube Topic channels may publish uploads only through the channel root.
- **Fix:** Keep `/videos` as the primary newest-first surface, but retry the validated channel root only when yt-dlp explicitly reports that the videos tab is absent.
- **Prevention check:** Test both a normal channel and a Topic channel against real yt-dlp, and keep a deterministic fake-process regression that proves unrelated yt-dlp failures do not trigger fallback.
- **Tags:** youtube, yt-dlp, channel, fallback, integration-testing
