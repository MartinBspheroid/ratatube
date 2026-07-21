# Product and Runtime Contract

This checked-in contract replaces references to an unavailable external PRD. Source comments retain their historic section numbers; the requirements below are the repository authority.

## 6. Prerequisites

`mpv` and `yt-dlp` are required. `curl` and `ffmpeg` are optional capabilities. Missing tools produce actionable diagnostics.

## 8–9. Layout and input

The application is keyboard-first and remains functional at 80x24. Primary views are Home, Search, Queue, Playlists, History, Now Playing, and Help. Channel is a contextual, non-tab destination that restores the previous view on Back. Help must expose reachable commands, support scrolling, and return to the opening view.

## 10. User workflows

Search accepts free text and structurally valid supported YouTube URLs. Exact video URLs preserve video identity. Queue order, repeat, shuffle, Previous history, local playlists, imports, resume, history, and metadata details must remain usable while external work is pending. Import reviews report exact rejection categories. Displayed listening time means advancing media-position seconds while playing, excluding pauses and seeks.

`c` opens a universal, non-blocking menu for the selected track on every track-bearing surface. Source-specific destructive actions target the exact queue or playlist occurrence. Channel browsing is newest-first, paginated in bounded 30-video pages, deduplicated across pages, retryable after partial failure, and leaves playback uninterrupted. The shared player presents a one-shot current-to-effective-next title transition in the final 15 seconds; it is disabled when no effective next track exists or repeat-track is active.

## 11. Local data

Configuration and application documents are local JSON with explicit schema versions, bounded size, atomic writes, strict validation, and future-version rejection. Runtime stream URLs are never persisted. Playlist deletion is local-only.

## 12–15. Playback and concurrency

One persistent `mpv` process receives correlated JSON IPC commands. External operations are supervised, cancellable, timeout-bounded, and stale-result safe. No external latency may block input dispatch. Shutdown cancels work, flushes persistence, and terminates owned processes within bounds.

## 16–18. Status and diagnostics

Loading, cancellation, failure, and non-durable state are explicit. Notifications expire by elapsed time, with longer visibility for errors. `doctor` is read-only and reports defaults-in-use, dependency status, and path/config failures without manufacturing health.

## 19. Security and resource limits

No shell interprets user input. Positional arguments follow `--`; URLs are structurally validated. Terminal control characters are removed from untrusted text. Process output, images, collections, files, retries, and timeouts are bounded. Sensitive URLs are redacted from logs.

## 20. Terminal compatibility

ASCII fallback is always available. Enhanced image/icon protocols are capability-dependent. Small terminals degrade explicitly instead of panicking.

## 26. Recovery

Malformed current data is preserved with recoverable evidence; unsupported future data is never rewritten. `mpv` disconnect triggers bounded recovery. Panic handling restores terminal state when process execution reaches the installed hook.

## 27. Background service

Playback runs in a background daemon owning `mpv`, `yt-dlp`, and all
persisted documents. `ytm` attaches the TUI as a control layer by default
(`--standalone` runs the historic single process) and transparently starts
the daemon when it is not running; playback continues after every client
detaches until `ytm quit` or SIGTERM. The control protocol is versioned
newline-delimited JSON over a mode-0600 Unix socket in the data directory,
bounded to the persistence frame limit. The bound socket is the
single-instance lock; a stale socket is probe-detected and removed. Any
number of clients may attach; broadcasts must never block the daemon, and a
client that cannot keep up is disconnected rather than buffered without
bound. Daemon loss under a live TUI triggers bounded respawn-and-reattach
with an explicit disconnected state. `ytm play`, `pause`, `stop`, `status`,
and `quit` are short-lived clients; `doctor` reports daemon liveness
read-only.
