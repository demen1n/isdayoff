//! Async client and builder.

use std::time::Duration;

use chrono::{Month, NaiveDate};

use crate::core::{self, Config, Query};
use crate::{Country, Day, DayType, Error};

/// Default timeout. Without one a request may hang forever: for example,
/// isdayoff.ru never answers a request with an unknown country code.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Async isdayoff.ru client.
///
/// Cloning is cheap (the inner `reqwest::Client` is an `Arc`), so build the
/// client once and pass clones around instead of rebuilding it per request.
///
/// ```no_run
/// use isdayoff::{IsDayOff, chrono::{Month, NaiveDate}};
///
/// # async fn run() -> Result<(), isdayoff::Error> {
/// let client = IsDayOff::builder().pre(true).holiday(true).build()?;
///
/// let new_year = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
/// println!("{}", client.day(new_year).await?); // holiday
///
/// for day in client.month(2025, Month::May).await? {
///     println!("{} — {}", day.date, day.kind);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct IsDayOff {
    config: Config,
    http: reqwest::Client,
}

impl IsDayOff {
    /// A client with default settings (Russia, no extra flags).
    pub fn new() -> Result<Self, Error> {
        Self::builder().build()
    }

    /// A builder for configuring the client.
    pub fn builder() -> IsDayOffBuilder {
        IsDayOffBuilder::default()
    }

    /// The type of a single day.
    pub async fn day(&self, date: NaiveDate) -> Result<DayType, Error> {
        let days = self.fetch(Query::Day(date)).await?;
        // `interpret` has already checked the length, but instead of `days[0]`
        // (a potential panic) we match on the slice pattern.
        match days.as_slice() {
            [day] => Ok(day.kind),
            other => Err(Error::LengthMismatch {
                expected: 1,
                actual: other.len(),
            }),
        }
    }

    /// All days of a month.
    pub async fn month(&self, year: i32, month: Month) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Month { year, month }).await
    }

    /// All days of a year (365 or 366).
    pub async fn year(&self, year: i32) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Year(year)).await
    }

    /// Days from `from` to `to`, inclusive.
    ///
    /// Returns [`Error::InvalidRange`] if `from > to`.
    pub async fn range(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Range { from, to }).await
    }

    async fn fetch(&self, query: Query) -> Result<Vec<Day>, Error> {
        let prepared = self.config.prepare(query)?;
        let response = self.http.get(prepared.url.clone()).send().await?;
        let status = response.status().as_u16();
        let body = response.text().await?;
        prepared.interpret(status, &body)
    }
}

/// Client builder. All flags are off by default, as in the API itself.
#[derive(Debug, Clone)]
pub struct IsDayOffBuilder {
    base_url: Option<String>,
    country: Country,
    pre: bool,
    sd: bool,
    covid: bool,
    holiday: bool,
    timeout: Duration,
    user_agent_comment: Option<String>,
}

impl Default for IsDayOffBuilder {
    fn default() -> Self {
        Self {
            base_url: None,
            country: Country::default(),
            pre: false,
            sd: false,
            covid: false,
            holiday: false,
            timeout: DEFAULT_TIMEOUT,
            user_agent_comment: None,
        }
    }
}

impl IsDayOffBuilder {
    /// Alternative service address (default `https://isdayoff.ru/`).
    /// Useful for tests with a local mock server or for a proxy.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Country (the `cc` parameter). Defaults to Russia.
    pub fn country(mut self, country: Country) -> Self {
        self.country = country;
        self
    }

    /// Mark shortened pre-holiday days as [`DayType::Shortened`].
    pub fn pre(mut self, enabled: bool) -> Self {
        self.pre = enabled;
        self
    }

    /// Use a six-day working week.
    pub fn six_day_week(mut self, enabled: bool) -> Self {
        self.sd = enabled;
        self
    }

    /// Mark the 2020 COVID non-working days as [`DayType::Covid`].
    pub fn covid(mut self, enabled: bool) -> Self {
        self.covid = enabled;
        self
    }

    /// Mark public holidays as [`DayType::Holiday`] instead of a plain day off.
    pub fn holiday(mut self, enabled: bool) -> Self {
        self.holiday = enabled;
        self
    }

    /// Timeout for the whole request (default 30 seconds).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Comment for the `User-Agent` header, e.g. how to contact the
    /// application's author. The isdayoff.ru author asks clients to set it.
    ///
    /// Resulting header: `isdayoff-rust-lib/<version> (+<repository>; <comment>)`.
    pub fn user_agent_comment(mut self, comment: impl Into<String>) -> Self {
        self.user_agent_comment = Some(comment.into());
        self
    }

    /// Builds the async client.
    pub fn build(self) -> Result<IsDayOff, Error> {
        let (config, user_agent, timeout) = self.into_parts()?;
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(timeout)
            .build()?;
        Ok(IsDayOff { config, http })
    }

    /// Build step shared by async and blocking clients: validates the settings.
    pub(crate) fn into_parts(self) -> Result<(Config, String, Duration), Error> {
        let base_url = self.base_url.as_deref().unwrap_or(core::DEFAULT_BASE_URL);
        let config = Config {
            base_url: url::Url::parse(base_url)?,
            country: self.country,
            pre: self.pre,
            sd: self.sd,
            covid: self.covid,
            holiday: self.holiday,
        };
        let user_agent = core::user_agent(self.user_agent_comment.as_deref());
        Ok((config, user_agent, self.timeout))
    }
}
