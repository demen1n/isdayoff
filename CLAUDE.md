# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust library crate wrapping the [isdayoff.ru](https://isdayoff.ru/) API — a Russian production calendar service that indicates whether a given date is a workday, day off, shortened day, or holiday.

## Commands

```sh
cargo build           # build the library
cargo test            # run all tests
cargo test <name>     # run a single test by name filter
cargo clippy          # lint
cargo fmt             # format
```

## Architecture

- `src/lib.rs` — crate root; modules are private, public API is re-exported via `pub use` (+ `pub use chrono`)
- `src/types.rs` — `Country`, `DayType` (`Workday`, `DayOff`, `Shortened`, `Covid`, `Holiday`), `Day { date, kind }`
- `src/error.rs` — `Error` (thiserror, `#[non_exhaustive]`)
- `src/core.rs` — sans-IO core shared by both clients: `Config::prepare(Query) -> Prepared` builds the URL, `Prepared::interpret(status, body)` parses the response. API errors (100/101/199) are distinguished from data by HTTP status, not body.
- `src/client.rs` — async `IsDayOff` + `IsDayOffBuilder` (timeout, User-Agent `isdayoff-rust-lib/<ver> (...)`)
- `src/blocking.rs` — `blocking::IsDayOff`, behind feature `blocking`; built via `IsDayOffBuilder::build_blocking()`
- `tests/` — mock HTTP server in `tests/common/mod.rs`; `tests/live.rs` hits the real API and is `#[ignore]`d (`cargo test --test live -- --ignored`)
- `edu/` — personal learning notes in Russian, git-ignored (local only); add a new numbered lesson when introducing a notable concept

Features: `rustls` (default), `native-tls`, `blocking`. Always check with `cargo test --all-features` and `cargo build --no-default-features`.
