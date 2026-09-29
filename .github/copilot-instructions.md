# AI assistant guidelines — EdgeFirst Recorder

Canonical process, CI tiers, runner policy and release chain:

https://github.com/EdgeFirstAI/.github/blob/main/.github/copilot-instructions.md

This file holds project-specific notes only. Do not duplicate branch, commit, PR or release rules here.

## Project-specific

### Layout

- Single binary crate, Linux only. `src/main.rs` (Zenoh subscription, writer loop), `src/args.rs` (CLI and Zenoh config), `src/clock.rs` (clock step detection, `clock_sync` state), `src/sink.rs` (MCAP output).
- `src/schemas/<pkg>/msg/<Type>.msg` are embedded at build time; `build.rs` generates `src/schemas.rs`, which is git-ignored and skipped by rustfmt.
- `ARCHITECTURE.md` documents the threading model, MCAP timestamps and the `clock_sync` / `clock_step` metadata contract.

### Tests

- Unit: `cargo nextest run --workspace --locked` (or `make test` for coverage). `tests/env_scrub.rs` is a `harness = false` binary that implements the nextest list protocol itself.
- A real clock step needs `CAP_SYS_TIME` and is tested on target (TESTING.md, "Clock Steps").
- Local end to end: the recorder's Zenoh session namespace is the hostname, so a test publisher must publish on `<hostname>/<topic>`.

### Platform notes

- Uses Linux-only APIs (`timerfd`, `adjtimex`, `clock_gettime` via `libc`); CI skips macOS and Windows.
- Release binaries are built with `cargo zigbuild` against glibc 2.17 for x86_64 and aarch64.
