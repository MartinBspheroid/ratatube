# ytm-tui

`ytm-tui` is a keyboard-first terminal application that searches YouTube, manages a local queue and playlists, and sends audio streams to a persistent `mpv` process. It is an unofficial client: it does not use a YouTube account, synchronize a library, remove ads, or guarantee that every video is playable.

## Requirements

- Rust 1.88 or newer; the repository pins the verified toolchain in `rust-toolchain.toml`.
- `mpv` and `yt-dlp` on `PATH`, or absolute executable paths in the config.
- `curl` is optional and enables thumbnails. `ffmpeg` is optional but may be required by `mpv` for some formats.
- An interactive terminal. The tested minimum is 80 columns by 24 rows; smaller terminals use a reduced layout.

Install platform packages first, then build:

```sh
cargo build --locked --release
./target/release/ytm-tui doctor
./target/release/ytm-tui
```

Use `/` to enter a search, type a query or supported YouTube URL, and press Enter. Select a result with `j`/`k` and press Enter to play. Press `?` for the complete scrollable command list. Immediate repeated `+`/`-` presses adjust volume cumulatively.

The command-line shortcut requires a query:

```sh
ytm-tui play massive attack teardrop
ytm-tui play 'https://www.youtube.com/watch?v=...'
ytm-tui --resume
```

## Data and configuration

Platform-native directories are selected by the `directories` crate. Run `ytm-tui doctor` to print the exact paths without creating or changing them. For isolated runs, `--data-dir PATH` stores both config and application data under `PATH`.

Copy `config.example.json` to the reported `config.json` path and edit only documented fields. Unknown fields, future schema versions, unsafe limits, and malformed JSON are rejected. `resumeOnLaunch` accepts `off`, `paused`, or `playing`; `icons` accepts `auto`, `nerd-font`, or `ascii`.

Runtime files include `queue.json`, `history.json`, `session.json`, `playlists/*.json`, `ytm-tui.log`, and at most one rotated `ytm-tui.log.1`. Documents are bounded to 16 MiB. Malformed migrated documents may receive a `.bak` copy; future-schema documents are left unchanged.

## Recovery and diagnostics

Start with:

```sh
ytm-tui doctor
RUST_LOG=debug ytm-tui
```

`doctor` is read-only. It reports missing dependencies, malformed config, and path problems but does not create directories, logs, or backups. If config is malformed, move it aside or repair the reported JSON; the interactive app otherwise starts with defaults and preserves the original. If queue/history/session persistence fails, the UI explicitly warns that recent changes are not durable.

If `mpv` exits, the app performs three bounded reconnect attempts and reports failure. If the terminal is left in raw mode after an external kill, run `reset`. Network operations can fail because of YouTube changes, regional restrictions, deleted/private media, or outdated `yt-dlp`; update `yt-dlp` and retry.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for exact gates, [ARCHITECTURE.md](ARCHITECTURE.md) for ownership and invariants, [PRD.md](PRD.md) for the checked-in behavioral contract, and [dependencies.md](dependencies.md) for dependency rationale.

This repository currently has no selected software license and is marked `publish = false`. Distribution rights must be decided by the owner before release.
