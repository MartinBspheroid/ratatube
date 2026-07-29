# Agent Guide

ratatube: a Rust ratatui YouTube-music player with a daemon/client split.
`CLAUDE.md` is a symlink to this file — edit here, both stay in sync.

## Workspace

Cargo workspace; run every command from the repository root so all members
are selected.

- `crates/ratatube-domain` — pure core (state, per-context commands, effects,
  events, models). No tokio, no ratatui, no process code; CI asserts the
  dependency tree stays clean.
- `crates/ratatube-protocol` — NDJSON frames and wire DTOs over the domain.
- `crates/ratatube-services` — the impure edge: mpv, yt-dlp, persistence,
  clipboard, child processes.
- `crates/ratatube-ui` — UI state, presentation reducers, rendering, keymap.
- `crates/ratatube` — the binary: daemon runtime, client runtime, CLI, wiring.

Services and UI are siblings and cannot name each other. Service and UI
modules re-export the domain model they adapt, so `crate::media::Track` and
friends still resolve from the binary.

## Read before changing code

- `ARCHITECTURE.md` — boundaries, action/reducer/effect flow, daemon
  protocol, persistence and security invariants.
- `DESIGN.md` — visual style and interaction language (theme roles, copy
  conventions, icon slots, breakpoints, mouse rules). Any UI or UX change
  must follow it; if a rule changes, update `DESIGN.md` in the same commit.
- `PRD.md` — the product/runtime contract; source comments cite its section
  numbers (e.g. "PRD 20").

## Verify gate (run before every commit)

```sh
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
git diff --check
```

- The suite is expected to pass every time, so treat any failure as a real
  regression instead of re-running it alone. The long-standing "different
  test fails every other run with Text file busy (os error 26)" flake was an
  `ETXTBSY` exec race in the script-based test doubles, not load sensitivity:
  a sibling thread's fork transiently inherits the write fd of the script
  another thread just wrote, and the owner's `exec` loses to it. Every fake
  executable is now created through `write_fake_executable`
  (`crates/ratatube-services/tests/support/fake_executable.rs`, twinned at
  `crates/ratatube/src/app/tests/fake_executable.rs`), which exec-probes the
  script until the window closes before the test uses it. Write new script
  doubles with that helper, never with a bare `fs::write` + chmod.
- `tests/mpv_ipc.rs` drives a real local mpv, so it needs `mpv` and `ffmpeg`
  on `PATH`; it skips itself when they are missing.
- Never pipe `cargo test` into `grep` in a `&&` chain without
  `set -o pipefail` — grep's exit code masks failures.
- Snapshot tests (`tests/ui_snapshots/`) pin rendered copy verbatim; a copy
  or layout change requires a same-commit sweep of those assertions.

## Knowledge graph (graphify)

`graphify-out/graph.json` is a knowledge graph of the whole repo (code via
AST, docs, and screenshots) with `GRAPH_REPORT.md` and a browsable
`graph.html`. Prefer it over grepping for architecture, call-flow, and
"what depends on what" questions:

```sh
/graphify query "How does a playlist rename flow from client to disk?"
/graphify path "PlaybackController" "MpvIpc"
/graphify explain "OperationRegistry"
/graphify . --update      # re-extract changed files after a refactor
```

The output directory is gitignored; if `graphify-out/graph.json` is missing,
build it with `/graphify .`.

## Conventions

- Action routing is wildcard-free on purpose, everywhere: adding a variant
  must force a routing decision at compile time. That covers
  `crates/ratatube/src/app/client_route.rs`,
  `crates/ratatube/src/app/service_actions/`,
  `crates/ratatube/src/app/reducer/` and `crates/ratatube-ui/src/reducer/`.
  Keep it that way — the catch-all that used to end the playlist storage
  handler silently dropped every by-id playlist command and shipped a real
  bug. Do not "fix" an exhaustiveness error by adding `_ =>`, and do not
  swap it for `unreachable!`: replacing a silent no-op with a crash is a
  worse bug. Where a sub-reducer only handles part of an enum, give it a
  narrowed input (a dedicated message type or a destructured payload) so the
  invalid case cannot be expressed — see `TrackContextMsg`.
- Daemon-side changes need a daemon restart to take effect in a live
  session: `./target/debug/ratatube quit --data-dir <data-dir>` — an
  attached TUI auto-respawns the daemon within seconds. Client-side changes
  need a TUI relaunch instead.
