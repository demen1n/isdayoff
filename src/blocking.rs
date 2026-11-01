//! Synchronous (blocking) client. Requires the `blocking` feature.
//!
//! ```toml
//! isdayoff = { version = "0.1", features = ["blocking"] }
//! ```
//!
//! **Important:** do not use this client inside async code (inside
//! `#[tokio::main]` etc.). `reqwest::blocking` spins up its own internal
//! runtime and panics if created or dropped inside another one.
//! In async code use [`crate::IsDayOff`].

use chrono::{Month, NaiveDate};

use crate::core::{Config, Query};
use crate::{Day, DayType, Error, IsDayOffBuilder};

/// Synchronous isdayoff.ru client. Same methods as [`crate::IsDayOff`],
/// just without `async`.
///
/// ```no_run
/// use isdayoff::{blocking::IsDayOff, chrono::NaiveDate};
///
/// let client = IsDayOff::new()?;
/// let today = NaiveDate::from_ymd_opt(2025, 1, 9).unwrap();
/// if client.day(today)?.is_working() {
///     println!("off to work");
/// }
/// # Ok::<(), isdayoff::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct IsDayOff {
    config: Config,
    http: reqwest::blocking::Client,
}

impl IsDayOff {
    /// A client with default settings.
    pub fn new() -> Result<Self, Error> {
        IsDayOffBuilder::default().build_blocking()
    }

    /// The builder (shared with the async client; finish with
    /// [`build_blocking`](IsDayOffBuilder::build_blocking)).
    pub fn builder() -> IsDayOffBuilder {
        IsDayOffBuilder::default()
    }

    /// The type of a single day.
    pub fn day(&self, date: NaiveDate) -> Result<DayType, Error> {
        match self.fetch(Query::Day(date))?.as_slice() {
            [day] => Ok(day.kind),
            other => Err(Error::LengthMismatch {
                expected: 1,
                actual: other.len(),
            }),
        }
    }

    /// All days of a month.
    pub fn month(&self, year: i32, month: Month) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Month { year, month })
    }

    /// All days of a year.
    pub fn year(&self, year: i32) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Year(year))
    }

    /// Days from `from` to `to`, inclusive.
    pub fn range(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Range { from, to })
    }

    fn fetch(&self, query: Query) -> Result<Vec<Day>, Error> {
        let prepared = self.config.prepare(query)?;
        let response = self.http.get(prepared.url.clone()).send()?;
        let status = response.status().as_u16();
        let body = response.text()?;
        prepared.interpret(status, &body)
    }
}

impl IsDayOffBuilder {
    /// Builds the synchronous client.
    pub fn build_blocking(self) -> Result<IsDayOff, Error> {
        let (config, user_agent, timeout) = self.into_parts()?;
        let http = reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .timeout(timeout)
            .build()?;
        Ok(IsDayOff { config, http })
    }
}
