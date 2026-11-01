//! Domain types: country, day type, dated day.

use std::fmt;

use chrono::NaiveDate;

use crate::Error;

/// Country whose production calendar is requested (the `cc` parameter).
///
/// Not every country known to isdayoff.ru is listed: data for Turkey, the USA
/// and Latvia ends in 2022, so they are not supported.
///
/// The enum is `#[non_exhaustive]`: adding a country in a future version
/// won't break downstream `match` expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Country {
    /// Russia (`ru`), the default.
    #[default]
    Russia,
    /// Belarus (`by`).
    Belarus,
    /// Kazakhstan (`kz`).
    Kazakhstan,
    /// Uzbekistan (`uz`).
    Uzbekistan,
}

impl Country {
    /// Two-letter country code (ISO 3166-1 alpha-2) as expected by the API.
    pub fn code(self) -> &'static str {
        match self {
            Country::Russia => "ru",
            Country::Belarus => "by",
            Country::Kazakhstan => "kz",
            Country::Uzbekistan => "uz",
        }
    }
}

impl fmt::Display for Country {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Day type — a single character of the API response.
///
/// [`Shortened`](DayType::Shortened), [`Covid`](DayType::Covid) and
/// [`Holiday`](DayType::Holiday) are returned **only** when the matching
/// builder flag (`pre`, `covid`, `holiday`) is enabled; otherwise such days
/// come back as plain `Workday`/`DayOff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DayType {
    /// `0` — a working day.
    Workday,
    /// `1` — a day off (non-working day).
    DayOff,
    /// `2` — a shortened pre-holiday working day (`pre` flag).
    Shortened,
    /// `4` — a COVID-19 non-working day, 2020 only (`covid` flag).
    Covid,
    /// `8` — a public holiday (`holiday` flag).
    Holiday,
}

impl DayType {
    /// Parses a single character of the API response.
    pub fn from_char(c: char) -> Result<Self, Error> {
        match c {
            '0' => Ok(DayType::Workday),
            '1' => Ok(DayType::DayOff),
            '2' => Ok(DayType::Shortened),
            '4' => Ok(DayType::Covid),
            '8' => Ok(DayType::Holiday),
            other => Err(Error::UnknownDayType(other)),
        }
    }

    /// Whether this is a working day (regular or shortened).
    pub fn is_working(self) -> bool {
        matches!(self, DayType::Workday | DayType::Shortened)
    }

    /// Whether this is a non-working day (day off, holiday, COVID day).
    pub fn is_day_off(self) -> bool {
        !self.is_working()
    }
}

impl fmt::Display for DayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DayType::Workday => "workday",
            DayType::DayOff => "day off",
            DayType::Shortened => "shortened day",
            DayType::Covid => "covid day off",
            DayType::Holiday => "holiday",
        })
    }
}

/// A specific date and its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Day {
    /// The date.
    pub date: NaiveDate,
    /// The day type.
    pub kind: DayType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_known_code() {
        let cases = [
            ('0', DayType::Workday),
            ('1', DayType::DayOff),
            ('2', DayType::Shortened),
            ('4', DayType::Covid),
            ('8', DayType::Holiday),
        ];
        for (c, expected) in cases {
            assert_eq!(DayType::from_char(c).unwrap(), expected, "code {c:?}");
        }
    }

    #[test]
    fn rejects_unknown_code() {
        assert!(matches!(
            DayType::from_char('3'),
            Err(Error::UnknownDayType('3'))
        ));
    }

    #[test]
    fn working_vs_day_off() {
        assert!(DayType::Workday.is_working());
        assert!(DayType::Shortened.is_working());
        assert!(DayType::DayOff.is_day_off());
        assert!(DayType::Holiday.is_day_off());
        assert!(DayType::Covid.is_day_off());
    }

    #[test]
    fn country_codes() {
        assert_eq!(Country::default().code(), "ru");
        assert_eq!(Country::Kazakhstan.to_string(), "kz");
    }
}
