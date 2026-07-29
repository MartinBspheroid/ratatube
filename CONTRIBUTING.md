# Contributing

Use the pinned Rust toolchain and keep `Cargo.lock` synchronized. Before changing a subsystem, read `ARCHITECTURE.md` and its colocated tests; UI changes must also follow `DESIGN.md`. Do not edit user data or depend on the machine's normal application directory in tests; use `--data-dir` and temporary directories.

The local merge gate is:

```sh
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
git diff --check
```

Unit, CLI, fake-process, and UI buffer tests are deterministic and must pass. `tests/mpv_ipc.rs` executes a real local `mpv` adapter test when prerequisites exist and prints an explicit skip otherwise. Three tests in `tests/yt_dlp.rs` use the live network and are `#[ignore]`; run them manually when adapter behavior changes:

```sh
cargo test --locked --test yt_dlp -- --ignored --nocapture
```

Add a regression test before fixing behavior. Keep external process tests behind fake binaries or local Unix sockets whenever possible. Never weaken byte/time limits or schema validation merely to accept a fixture. Review `git diff` before committing and document any check that could not run.

Generated/runtime artifacts (`target`, `.DS_Store`, local `ruvector.db`) are ignored. No generated source is checked in. License terms are unresolved; do not publish or redistribute the package until the owner chooses them.
