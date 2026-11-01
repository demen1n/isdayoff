//! Client for the [isdayoff.ru](https://isdayoff.ru/) production calendar API.
//!
//! Tells whether a date is a working day, a day off, a shortened pre-holiday
//! day or a public holiday — for Russia, Belarus, Kazakhstan and Uzbekistan.
//!
//! # Quick start
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
//!     println!("{date}: {kind}, working: {}", kind.is_working());
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! | feature      | default | description                                      |
//! |--------------|:-------:|--------------------------------------------------|
//! | `rustls`     | yes     | pure-Rust TLS                                    |
//! | `native-tls` | no      | system TLS (OpenSSL / Secure Transport / SChannel) |
//! | `blocking`   | no      | synchronous client [`blocking::IsDayOff`]        |
//!
//! The async client's API isn't tied to a particular runtime, but `reqwest`
//! uses `tokio` under the hood, so call it from within a tokio runtime.

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

/// Re-export of `chrono`: dates in this crate's API are `chrono::NaiveDate`
/// and `chrono::Month`, so users don't have to pick a compatible version.
pub use chrono;
