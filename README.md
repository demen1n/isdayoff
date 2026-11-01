# isdayoff

[![crates.io](https://img.shields.io/crates/v/isdayoff.svg)](https://crates.io/crates/isdayoff)
[![docs.rs](https://docs.rs/isdayoff/badge.svg)](https://docs.rs/isdayoff)

**English** | [Русский](README.ru.md)

Rust client for the [isdayoff.ru](https://isdayoff.ru/) production calendar API:
workdays, days off, shortened pre-holiday days and public holidays for Russia,
Belarus, Kazakhstan and Uzbekistan.

```toml
[dependencies]
isdayoff = "0.1"
# synchronous client:
# isdayoff = { version = "0.1", features = ["blocking"] }
```

## Example

```rust,no_run
use isdayoff::{IsDayOff, chrono::{Month, NaiveDate}};

#[tokio::main]
async fn main() -> Result<(), isdayoff::Error> {
    let client = IsDayOff::builder()
        .pre(true)        // shortened days -> DayType::Shortened
        .holiday(true)    // holidays -> DayType::Holiday
        .user_agent_comment("my-app, me@example.com")
        .build()?;

    let kind = client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()).await?;
    println!("January 1st: {kind}");

    let may = client.month(2025, Month::May).await?;
    let working = may.iter().filter(|d| d.kind.is_working()).count();
    println!("working days in May 2025: {working}");
    Ok(())
}
```

Client methods: `day`, `month`, `year`, `range(from, to)`.

## Features

| feature      | default | description |
|--------------|:---:|---|
| `rustls`     | ✓ | pure-Rust TLS |
| `native-tls` |   | system TLS |
| `blocking`   |   | synchronous client `isdayoff::blocking::IsDayOff` |

## User-Agent

As requested by the service author, the client sends
`User-Agent: isdayoff-rust-lib/<version> (+https://github.com/demen1n/isdayoff; <comment>)`.
Set the comment with `.user_agent_comment(...)` — put your contact info there.

## Development

```sh
cargo test --all-features                 # unit + mock-server tests, no network
cargo test --test live -- --ignored       # against the real isdayoff.ru
cargo run --example async
cargo run --example blocking --features blocking
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
