# Agent Guide

ratatube: a Rust ratatui YouTube-music player with a daemon/client split.
`CLAUDE.md` is a symlink to this file — edit here, both stay in sync.

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

- Run one `cargo test` invocation at a time — `tests/mpv_ipc.rs` drives a
  real local mpv and flakes under parallel load; re-run a failure alone
  before treating it as a regression.
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

- `src/app/client_route.rs` and `src/app/service_actions/` match
  wildcard-free on purpose: adding an action variant forces a routing
  decision at compile time. Keep it that way — the catch-all that used to
  end the playlist storage handler silently dropped every by-id playlist
  command and shipped a real bug.
- Daemon-side changes need a daemon restart to take effect in a live
  session: `./target/debug/ratatube quit --data-dir <data-dir>` — an
  attached TUI auto-respawns the daemon within seconds. Client-side changes
  need a TUI relaunch instead.
