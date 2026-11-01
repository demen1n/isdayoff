//! Асинхронный клиент и билдер.

use std::time::Duration;

use chrono::{Month, NaiveDate};

use crate::core::{self, Config, Query};
use crate::{Country, Day, DayType, Error};

/// Таймаут по умолчанию. Без него запрос может висеть бесконечно:
/// например, isdayoff.ru не отвечает вовсе на неизвестный код страны.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Асинхронный клиент isdayoff.ru.
///
/// Клиент дёшево клонируется (`reqwest::Client` внутри — это `Arc`), поэтому
/// создавайте его один раз и передавайте копии, а не собирайте заново на
/// каждый запрос.
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
    /// Клиент с настройками по умолчанию (Россия, без доп. флагов).
    pub fn new() -> Result<Self, Error> {
        Self::builder().build()
    }

    /// Билдер для настройки клиента.
    pub fn builder() -> IsDayOffBuilder {
        IsDayOffBuilder::default()
    }

    /// Тип одного дня.
    pub async fn day(&self, date: NaiveDate) -> Result<DayType, Error> {
        let days = self.fetch(Query::Day(date)).await?;
        // Длину ответа уже проверил `interpret`, но вместо `days[0]`
        // (потенциальная паника) честно сопоставляем с образцом.
        match days.as_slice() {
            [day] => Ok(day.kind),
            other => Err(Error::LengthMismatch {
                expected: 1,
                actual: other.len(),
            }),
        }
    }

    /// Все дни месяца.
    pub async fn month(&self, year: i32, month: Month) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Month { year, month }).await
    }

    /// Все дни года (365 или 366).
    pub async fn year(&self, year: i32) -> Result<Vec<Day>, Error> {
        self.fetch(Query::Year(year)).await
    }

    /// Дни с `from` по `to` включительно.
    ///
    /// Возвращает [`Error::InvalidRange`], если `from > to`.
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

/// Билдер клиента. Все флаги по умолчанию выключены — как и в самом API.
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
    /// Другой адрес сервиса (по умолчанию `https://isdayoff.ru/`).
    /// Полезно для тестов с локальным mock-сервером или для прокси.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Страна (параметр `cc`). По умолчанию — Россия.
    pub fn country(mut self, country: Country) -> Self {
        self.country = country;
        self
    }

    /// Помечать сокращённые предпраздничные дни как [`DayType::Shortened`].
    pub fn pre(mut self, enabled: bool) -> Self {
        self.pre = enabled;
        self
    }

    /// Считать неделю шестидневной.
    pub fn six_day_week(mut self, enabled: bool) -> Self {
        self.sd = enabled;
        self
    }

    /// Помечать «нерабочие дни» 2020 года как [`DayType::Covid`].
    pub fn covid(mut self, enabled: bool) -> Self {
        self.covid = enabled;
        self
    }

    /// Помечать праздники как [`DayType::Holiday`], а не как обычный выходной.
    pub fn holiday(mut self, enabled: bool) -> Self {
        self.holiday = enabled;
        self
    }

    /// Таймаут на весь запрос (по умолчанию 30 секунд).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Комментарий в заголовке `User-Agent`, например как связаться с
    /// автором приложения. Автор isdayoff.ru просит его указывать.
    ///
    /// Итоговый заголовок: `isdayoff-rust-lib/<версия> (+<репозиторий>; <комментарий>)`.
    pub fn user_agent_comment(mut self, comment: impl Into<String>) -> Self {
        self.user_agent_comment = Some(comment.into());
        self
    }

    /// Собирает асинхронный клиент.
    pub fn build(self) -> Result<IsDayOff, Error> {
        let (config, user_agent, timeout) = self.into_parts()?;
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(timeout)
            .build()?;
        Ok(IsDayOff { config, http })
    }

    /// Общая для async и blocking часть сборки: валидирует настройки.
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
