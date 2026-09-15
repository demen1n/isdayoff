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

- `src/lib.rs` — crate root (not yet created; will expose the public API)
- `src/types.rs` — core types (`DayType` enum: `Workday`, `Dayoff`, `Shortened`, `Holiday`)

The library will need an HTTP client dependency (e.g. `reqwest`) to call the isdayoff.ru REST API. The API returns single-character or multi-character digit strings indicating day types for a given date or date range.
