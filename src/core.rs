//! Sans-IO core: everything that can be done without the network.
//!
//! The async and blocking clients differ only in *how* they send the HTTP
//! request. Building the URL and parsing the response are identical, so they
//! live here and are covered by plain synchronous unit tests — no server,
//! no runtime.

use chrono::{Month, Months, NaiveDate};
use url::Url;

use crate::{Country, Day, DayType, Error};

pub(crate) const DEFAULT_BASE_URL: &str = "https://isdayoff.ru/";

/// Body size limit for non-200 responses: the API's error codes are three
/// bytes, anything longer is only kept for the error message anyway.
const ERROR_BODY_LIMIT: usize = 1024;

/// Parses the base URL and makes sure its path ends with `/`.
///
/// `Url::join` treats the last path segment without a trailing slash as a
/// "file" and replaces it: `https://proxy/isdayoff` + `api/getdata` would give
/// `https://proxy/api/getdata`, silently dropping `isdayoff`. Appending the
/// slash keeps the whole prefix.
pub(crate) fn parse_base_url(base_url: &str) -> Result<Url, Error> {
    let mut url = Url::parse(base_url)?;
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

/// Request settings shared by the async and blocking clients.
#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) base_url: Url,
    pub(crate) country: Country,
    pub(crate) pre: bool,
    pub(crate) sd: bool,
    pub(crate) covid: bool,
    pub(crate) holiday: bool,
}

/// What exactly we ask the API for.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Query {
    Day(NaiveDate),
    Month { year: i32, month: Month },
    Year(i32),
    Range { from: NaiveDate, to: NaiveDate },
}

/// A request ready to be sent, plus what is needed to parse its response.
#[derive(Debug)]
pub(crate) struct Prepared {
    pub(crate) url: Url,
    /// First date of the period; the first character of the response maps to it.
    first: NaiveDate,
    /// How many days (characters) the response must contain.
    len: usize,
}

impl Query {
    /// First and last day of the period (inclusive).
    fn span(self) -> Result<(NaiveDate, NaiveDate), Error> {
        const OUT_OF_RANGE: Error = Error::InvalidRange("date is out of supported range");

        match self {
            Query::Day(date) => Ok((date, date)),
            Query::Month { year, month } => {
                let first = NaiveDate::from_ymd_opt(year, month.number_from_month(), 1)
                    .ok_or(OUT_OF_RANGE)?;
                // Last day of the month = (first day of the next month) - 1 day.
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
    /// Builds the request URL and records how many days to expect back.
    pub(crate) fn prepare(&self, query: Query) -> Result<Prepared, Error> {
        let (first, last) = query.span()?;
        // `num_days()` is i64; the period from `span()` is never negative.
        let len = usize::try_from((last - first).num_days() + 1)
            .map_err(|_| Error::InvalidRange("period is too long"))?;

        // Relative path without a leading `/`: with base_url = "http://proxy/isdayoff/"
        // this yields ".../isdayoff/api/getdata", not "http://proxy/api/getdata".
        let mut url = self.base_url.join("api/getdata")?;

        // `q` holds `&mut url` and implements Drop, so the borrow lasts until the
        // end of `q`'s scope — hence the separate block.
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
    /// Maximum number of body bytes worth reading for a response with this status.
    ///
    /// Clients read at most `body_limit + 1` bytes: the extra byte is how
    /// [`interpret`](Self::interpret) learns that the body was cut off.
    /// This bounds memory use even if a broken server or proxy sends gigabytes.
    pub(crate) fn body_limit(&self, status: u16) -> usize {
        if status == 200 {
            // One character per day; up to 4 bytes per char in UTF-8 so that an
            // unexpected non-ASCII body still yields a precise error, plus some
            // room for surrounding whitespace (the body is trimmed).
            self.len.saturating_mul(4).saturating_add(64)
        } else {
            ERROR_BODY_LIMIT
        }
    }

    /// Turns the HTTP status and response body into a list of days.
    ///
    /// `body` is what the client read, at most `body_limit(status) + 1` bytes.
    ///
    /// Why the status matters and not just the body: a response for a
    /// three-day range can legitimately be `"100"` (day off, workday, workday),
    /// exactly like the "bad date" error code. Only the HTTP status tells them
    /// apart: data comes with `200`, errors with `400`/`404`/...
    pub(crate) fn interpret(&self, status: u16, body: &[u8]) -> Result<Vec<Day>, Error> {
        let limit = self.body_limit(status);
        let too_large = body.len() > limit;

        if status == 200 && too_large {
            return Err(Error::ResponseTooLarge { limit });
        }
        // An oversized error body is just truncated: it only goes into the message.
        let body = &body[..body.len().min(limit)];
        // Invalid UTF-8 becomes U+FFFD and is then reported as an unknown day type.
        let body = String::from_utf8_lossy(body);
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
        // Count chars, not bytes (`len()`), so the error stays accurate even if
        // the server unexpectedly sends non-ASCII.
        let actual = body.chars().count();
        if actual != self.len {
            return Err(Error::LengthMismatch {
                expected: self.len,
                actual,
            });
        }

        // Dates from `first` onwards. Not `first.iter_days()`: it stops *before*
        // `NaiveDate::MAX`, so a range ending on that date would silently lose
        // its last day. `successors` yields MAX and then ends (`succ_opt` is None).
        // The period from `span()` always fits, so there are exactly `len` dates
        // and `zip` consumes every character.
        // `collect::<Result<Vec<_>, _>>()` short-circuits on the first error.
        let dates = std::iter::successors(Some(self.first), |d| d.succ_opt());
        body.chars()
            .zip(dates)
            .map(|(c, date)| {
                Ok(Day {
                    date,
                    kind: DayType::from_char(c)?,
                })
            })
            .collect()
    }
}

/// `User-Agent` header value in the format requested by the API author:
/// `isdayoff-<language>-lib/<version> (<comment>)`.
pub(crate) fn user_agent(comment: Option<&str>) -> String {
    // `env!` substitutes values from Cargo.toml at compile time.
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
            base_url: parse_base_url(DEFAULT_BASE_URL).unwrap(),
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
        assert_eq!(p.len, 29, "2024 is a leap year");
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
        assert_eq!(p.len, 4, "bounds are inclusive");
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
    fn base_url_without_trailing_slash_keeps_last_segment() {
        for base in [
            "https://proxy.example/isdayoff",
            "https://proxy.example/isdayoff/",
        ] {
            let cfg = Config {
                base_url: parse_base_url(base).unwrap(),
                ..config()
            };
            let p = cfg.prepare(Query::Year(2025)).unwrap();
            assert_eq!(
                p.url.as_str(),
                "https://proxy.example/isdayoff/api/getdata?year=2025&cc=ru",
                "base = {base}"
            );
        }
        // A bare host gets the root path "/" from the URL parser.
        assert_eq!(
            parse_base_url("http://localhost:8080").unwrap().as_str(),
            "http://localhost:8080/"
        );
    }

    #[test]
    fn range_ending_on_max_date_keeps_last_day() {
        let max = NaiveDate::MAX;
        let p = config()
            .prepare(Query::Range {
                from: max.pred_opt().unwrap(),
                to: max,
            })
            .unwrap();
        let days = p.interpret(200, b"01").unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(days[1].date, max);
        assert_eq!(days[1].kind, DayType::DayOff);
    }

    #[test]
    fn range_starting_on_min_date() {
        let min = NaiveDate::MIN;
        let p = config()
            .prepare(Query::Range {
                from: min,
                to: min.succ_opt().unwrap(),
            })
            .unwrap();
        let days = p.interpret(200, b"10").unwrap();
        assert_eq!(days[0].date, min);
        assert_eq!(days.len(), 2);
    }

    #[test]
    fn year_out_of_chrono_range_is_rejected() {
        let err = config().prepare(Query::Year(i32::MAX)).unwrap_err();
        assert!(matches!(err, Error::InvalidRange(_)));
    }

    #[test]
    fn oversized_data_body_is_rejected() {
        let p = config().prepare(Query::Day(date(2025, 1, 1))).unwrap();
        let limit = p.body_limit(200);
        // Exactly at the limit: still parsed (and fails on length, not size).
        let mut at_limit = vec![b' '; limit - 1];
        at_limit.push(b'1');
        assert!(matches!(p.interpret(200, &at_limit), Ok(days) if days.len() == 1));
        let over = vec![b'0'; limit + 1];
        assert!(matches!(
            p.interpret(200, &over),
            Err(Error::ResponseTooLarge { limit: l }) if l == limit
        ));
    }

    #[test]
    fn oversized_error_body_is_truncated() {
        let p = config().prepare(Query::Day(date(2025, 1, 1))).unwrap();
        let body = vec![b'x'; p.body_limit(502) + 1];
        match p.interpret(502, &body) {
            Err(Error::UnexpectedResponse { status: 502, body }) => {
                assert_eq!(body.chars().count(), 200)
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn non_utf8_body_is_an_unknown_day_type() {
        let p = config().prepare(Query::Day(date(2025, 1, 1))).unwrap();
        assert!(matches!(
            p.interpret(200, &[0xFF]),
            Err(Error::UnknownDayType('\u{FFFD}'))
        ));
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
        let days = p.interpret(200, b"100").unwrap();
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
        assert!(matches!(p.interpret(400, b"100"), Err(Error::BadRequest)));
        assert!(matches!(p.interpret(404, b"101"), Err(Error::NotFound)));
        assert!(matches!(p.interpret(500, b"199"), Err(Error::ServiceError)));
        assert!(matches!(
            p.interpret(502, b"<html>Bad Gateway</html>"),
            Err(Error::UnexpectedResponse { status: 502, .. })
        ));
    }

    #[test]
    fn length_mismatch_is_detected() {
        let p = config().prepare(Query::Year(2025)).unwrap();
        assert!(matches!(
            p.interpret(200, b"0101"),
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
