# isdayoff

[![crates.io](https://img.shields.io/crates/v/isdayoff.svg)](https://crates.io/crates/isdayoff)
[![docs.rs](https://docs.rs/isdayoff/badge.svg)](https://docs.rs/isdayoff)

Rust-клиент API производственного календаря [isdayoff.ru](https://isdayoff.ru/):
рабочие дни, выходные, сокращённые предпраздничные дни и праздники для России,
Беларуси, Казахстана и Узбекистана.

```toml
[dependencies]
isdayoff = "0.1"
# синхронный клиент:
# isdayoff = { version = "0.1", features = ["blocking"] }
```

## Пример

```rust,no_run
use isdayoff::{IsDayOff, chrono::{Month, NaiveDate}};

#[tokio::main]
async fn main() -> Result<(), isdayoff::Error> {
    let client = IsDayOff::builder()
        .pre(true)        // сокращённые дни -> DayType::Shortened
        .holiday(true)    // праздники -> DayType::Holiday
        .user_agent_comment("my-app, me@example.com")
        .build()?;

    let kind = client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()).await?;
    println!("1 января: {kind}");

    let may = client.month(2025, Month::May).await?;
    let working = may.iter().filter(|d| d.kind.is_working()).count();
    println!("в мае 2025 рабочих дней: {working}");
    Ok(())
}
```

Методы клиента: `day`, `month`, `year`, `range(from, to)`.

## Фичи

| фича         | по умолчанию | описание |
|--------------|:---:|---|
| `rustls`     | ✓ | TLS на чистом Rust |
| `native-tls` |   | системный TLS |
| `blocking`   |   | синхронный клиент `isdayoff::blocking::IsDayOff` |

## User-Agent

По просьбе автора сервиса клиент отправляет заголовок
`User-Agent: isdayoff-rust-lib/<версия> (+https://github.com/demen1n/isdayoff; <комментарий>)`.
Комментарий задаётся через `.user_agent_comment(...)` — укажите там, как с вами связаться.

## Разработка

```sh
cargo test --all-features                 # unit + тесты с mock-сервером, без сети
cargo test --test live -- --ignored       # против настоящего isdayoff.ru
cargo run --example async
cargo run --example blocking --features blocking
```


## Лицензия

MIT или Apache-2.0 на ваш выбор.
