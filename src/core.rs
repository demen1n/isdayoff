//! «Ядро без ввода-вывода» (sans-IO): всё, что можно сделать без сети.
//!
//! Async- и blocking-клиенты отличаются только тем, *как* они отправляют
//! HTTP-запрос. Построение URL и разбор ответа одинаковы, поэтому живут здесь
//! и тестируются обычными синхронными unit-тестами, без сервера и рантайма.

use chrono::{Month, Months, NaiveDate};
use url::Url;

use crate::{Country, Day, DayType, Error};

pub(crate) const DEFAULT_BASE_URL: &str = "https://isdayoff.ru/";

/// Настройки запросов, общие для async- и blocking-клиента.
#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) base_url: Url,
    pub(crate) country: Country,
    pub(crate) pre: bool,
    pub(crate) sd: bool,
    pub(crate) covid: bool,
    pub(crate) holiday: bool,
}

/// Что именно спрашиваем у API.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Query {
    Day(NaiveDate),
    Month { year: i32, month: Month },
    Year(i32),
    Range { from: NaiveDate, to: NaiveDate },
}

/// Готовый к отправке запрос + то, что нужно знать для разбора ответа.
#[derive(Debug)]
pub(crate) struct Prepared {
    pub(crate) url: Url,
    /// Первая дата периода — к ней «привязываем» первый символ ответа.
    first: NaiveDate,
    /// Сколько дней (символов) должно прийти.
    len: usize,
}

impl Query {
    /// Первый и последний день периода (включительно).
    fn span(self) -> Result<(NaiveDate, NaiveDate), Error> {
        const OUT_OF_RANGE: Error = Error::InvalidRange("date is out of supported range");

        match self {
            Query::Day(date) => Ok((date, date)),
            Query::Month { year, month } => {
                let first = NaiveDate::from_ymd_opt(year, month.number_from_month(), 1)
                    .ok_or(OUT_OF_RANGE)?;
                // Последний день месяца = (первое число следующего месяца) - 1 день.
                let last = first
                    .checked_add_months(Months::new(1))
                    .and_then(|d| d.pred_opt())
                    .ok_or(OUT_OF_RANGE)?;
                Ok((first, last))
            }
            Query::Year(year) => {
                let first = NaiveDate::from_ymd_opt(year, 1, 1).ok_or(OUT_OF_RANGE)?;
                let last = NaiveDate::from_ymd_opt(year, 12, 31).ok_or(OUT_OF_RANGE)?;
                Ok((first, last))
            }
            Query::Range { from, to } if from > to => {
                Err(Error::InvalidRange("`from` is later than `to`"))
            }
            Query::Range { from, to } => Ok((from, to)),
        }
    }
}

impl Config {
    /// Строит URL запроса и запоминает, сколько дней ждать в ответе.
    pub(crate) fn prepare(&self, query: Query) -> Result<Prepared, Error> {
        let (first, last) = query.span()?;
        // `num_days()` — i64; период из `span()` никогда не отрицательный.
        let len = usize::try_from((last - first).num_days() + 1)
            .map_err(|_| Error::InvalidRange("period is too long"))?;

        // Относительный путь без ведущего `/`: если base_url = "http://proxy/isdayoff/",
        // получится ".../isdayoff/api/getdata", а не "http://proxy/api/getdata".
        let mut url = self.base_url.join("api/getdata")?;

        // `q` держит `&mut url` и имеет деструктор (Drop): заимствование живёт до
        // конца области видимости `q`, поэтому — отдельный блок.
        {
            let mut q = url.query_pairs_mut();
            match query {
                Query::Day(date) => {
                    q.append_pair("year", &date.format("%Y").to_string());
                    q.append_pair("month", &date.format("%m").to_string());
                    q.append_pair("day", &date.format("%d").to_string());
                }
                Query::Month { year, month } => {
                    q.append_pair("year", &year.to_string());
                    q.append_pair("month", &format!("{:02}", month.number_from_month()));
                }
                Query::Year(year) => {
                    q.append_pair("year", &year.to_string());
                }
                Query::Range { from, to } => {
                    q.append_pair("date1", &from.format("%Y%m%d").to_string());
                    q.append_pair("date2", &to.format("%Y%m%d").to_string());
                }
            }
            q.append_pair("cc", self.country.code());
            for (name, enabled) in [
                ("pre", self.pre),
                ("sd", self.sd),
                ("covid", self.covid),
                ("holiday", self.holiday),
            ] {
                if enabled {
                    q.append_pair(name, "1");
                }
            }
        }

        Ok(Prepared { url, first, len })
    }
}

impl Prepared {
    /// Превращает HTTP-статус и тело ответа в список дней.
    ///
    /// Почему нужен статус, а не только тело: ответ за диапазон из трёх дней
    /// легально может быть строкой `"100"` (рабочий, выходной, выходной) —
    /// ровно как код ошибки «неверная дата». Отличить их можно только по
    /// HTTP-статусу: данные приходят с `200`, ошибки — с `400`/`404`/...
    pub(crate) fn interpret(&self, status: u16, body: &str) -> Result<Vec<Day>, Error> {
        let body = body.trim();

        if status == 200 {
            return self.parse_days(body);
        }

        Err(match body {
            "100" => Error::BadRequest,
            "101" => Error::NotFound,
            "199" => Error::ServiceError,
            other => Error::UnexpectedResponse {
                status,
                body: other.chars().take(200).collect(),
            },
        })
    }

    fn parse_days(&self, body: &str) -> Result<Vec<Day>, Error> {
        // Считаем символы, а не байты (`len()`): так ошибка будет честной,
        // даже если сервер вдруг пришлёт не-ASCII.
        let actual = body.chars().count();
        if actual != self.len {
            return Err(Error::LengthMismatch {
                expected: self.len,
                actual,
            });
        }

        // `iter_days()` — бесконечный итератор дат, начиная с `first`;
        // `zip` остановится, когда закончатся символы ответа.
        // `collect::<Result<Vec<_>, _>>()` прервётся на первой ошибке.
        body.chars()
            .zip(self.first.iter_days())
            .map(|(c, date)| {
                Ok(Day {
                    date,
                    kind: DayType::from_char(c)?,
                })
            })
            .collect()
    }
}

/// Значение заголовка `User-Agent` в формате, о котором просит автор API:
/// `isdayoff-<язык>-lib/<версия> (<комментарий>)`.
pub(crate) fn user_agent(comment: Option<&str>) -> String {
    // `env!` подставляет значения из Cargo.toml на этапе компиляции.
    let version = env!("CARGO_PKG_VERSION");
    let repo = env!("CARGO_PKG_REPOSITORY");
    match comment {
        Some(c) => format!("isdayoff-rust-lib/{version} (+{repo}; {c})"),
        None => format!("isdayoff-rust-lib/{version} (+{repo})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            base_url: Url::parse(DEFAULT_BASE_URL).unwrap(),
            country: Country::Russia,
            pre: false,
            sd: false,
            covid: false,
            holiday: false,
        }
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn url_for_day() {
        let p = config().prepare(Query::Day(date(2025, 1, 9))).unwrap();
        assert_eq!(
            p.url.as_str(),
            "https://isdayoff.ru/api/getdata?year=2025&month=01&day=09&cc=ru"
        );
        assert_eq!(p.len, 1);
    }

    #[test]
    fn url_for_month_with_flags() {
        let cfg = Config {
            country: Country::Belarus,
            pre: true,
            holiday: true,
            ..config()
        };
        let p = cfg
            .prepare(Query::Month {
                year: 2024,
                month: Month::February,
            })
            .unwrap();
        assert_eq!(
            p.url.as_str(),
            "https://isdayoff.ru/api/getdata?year=2024&month=02&cc=by&pre=1&holiday=1"
        );
        assert_eq!(p.len, 29, "2024 — високосный");
    }

    #[test]
    fn url_for_range() {
        let p = config()
            .prepare(Query::Range {
                from: date(2025, 12, 30),
                to: date(2026, 1, 2),
            })
            .unwrap();
        assert_eq!(
            p.url.as_str(),
            "https://isdayoff.ru/api/getdata?date1=20251230&date2=20260102&cc=ru"
        );
        assert_eq!(p.len, 4, "границы включительно");
    }

    #[test]
    fn year_lengths() {
        assert_eq!(config().prepare(Query::Year(2023)).unwrap().len, 365);
        assert_eq!(config().prepare(Query::Year(2024)).unwrap().len, 366);
    }

    #[test]
    fn base_url_with_path_is_kept() {
        let cfg = Config {
            base_url: Url::parse("http://localhost:8080/proxy/").unwrap(),
            ..config()
        };
        let p = cfg.prepare(Query::Year(2025)).unwrap();
        assert!(
            p.url
                .as_str()
                .starts_with("http://localhost:8080/proxy/api/getdata?")
        );
    }

    #[test]
    fn reversed_range_is_rejected() {
        let err = config()
            .prepare(Query::Range {
                from: date(2025, 2, 1),
                to: date(2025, 1, 1),
            })
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRange(_)));
    }

    #[test]
    fn interprets_data_and_assigns_dates() {
        let p = config()
            .prepare(Query::Range {
                from: date(2025, 1, 7),
                to: date(2025, 1, 9),
            })
            .unwrap();
        let days = p.interpret(200, "100").unwrap();
        assert_eq!(
            days,
            vec![
                Day {
                    date: date(2025, 1, 7),
                    kind: DayType::DayOff
                },
                Day {
                    date: date(2025, 1, 8),
                    kind: DayType::Workday
                },
                Day {
                    date: date(2025, 1, 9),
                    kind: DayType::Workday
                },
            ]
        );
    }

    #[test]
    fn same_body_different_status_is_an_error() {
        let p = config()
            .prepare(Query::Range {
                from: date(2025, 1, 7),
                to: date(2025, 1, 9),
            })
            .unwrap();
        assert!(matches!(p.interpret(400, "100"), Err(Error::BadRequest)));
        assert!(matches!(p.interpret(404, "101"), Err(Error::NotFound)));
        assert!(matches!(p.interpret(500, "199"), Err(Error::ServiceError)));
        assert!(matches!(
            p.interpret(502, "<html>Bad Gateway</html>"),
            Err(Error::UnexpectedResponse { status: 502, .. })
        ));
    }

    #[test]
    fn length_mismatch_is_detected() {
        let p = config().prepare(Query::Year(2025)).unwrap();
        assert!(matches!(
            p.interpret(200, "0101"),
            Err(Error::LengthMismatch {
                expected: 365,
                actual: 4
            })
        ));
    }

    #[test]
    fn user_agent_format() {
        let ua = user_agent(Some("me@example.com"));
        assert!(ua.starts_with(&format!(
            "isdayoff-rust-lib/{} (",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(ua.ends_with("; me@example.com)"));
    }
}
