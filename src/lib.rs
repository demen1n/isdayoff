//! Клиент API производственного календаря [isdayoff.ru](https://isdayoff.ru/).
//!
//! Позволяет узнать, является ли дата рабочим днём, выходным, сокращённым
//! предпраздничным днём или праздником — для России, Беларуси, Казахстана
//! и Узбекистана.
//!
//! # Быстрый старт
//!
//! ```no_run
//! use isdayoff::{IsDayOff, chrono::NaiveDate};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), isdayoff::Error> {
//!     let client = IsDayOff::builder()
//!         .holiday(true)
//!         .user_agent_comment("my-app, me@example.com")
//!         .build()?;
//!
//!     let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
//!     let kind = client.day(date).await?;
//!     println!("{date}: {kind}, работаем: {}", kind.is_working());
//!     Ok(())
//! }
//! ```
//!
//! # Фичи
//!
//! | фича         | по умолчанию | что даёт                                         |
//! |--------------|:------------:|--------------------------------------------------|
//! | `rustls`     | да           | TLS на чистом Rust                               |
//! | `native-tls` | нет          | системный TLS (OpenSSL / Secure Transport / SChannel) |
//! | `blocking`   | нет          | синхронный клиент [`blocking::IsDayOff`]         |
//!
//! Async-клиент не привязан к конкретному рантайму в API, но `reqwest`
//! под капотом использует `tokio`, поэтому вызывайте его из tokio-рантайма.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod client;
mod core;
mod error;
mod types;

#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub mod blocking;

pub use client::{IsDayOff, IsDayOffBuilder};
pub use error::Error;
pub use types::{Country, Day, DayType};

/// Реэкспорт `chrono`: даты в API крейта — это `chrono::NaiveDate` и
/// `chrono::Month`. Так пользователю не нужно подбирать совместимую версию.
pub use chrono;
