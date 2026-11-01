//! Предметные типы: страна, тип дня, день с датой.

use std::fmt;

use chrono::NaiveDate;

use crate::Error;

/// Страна, для которой запрашивается производственный календарь (параметр `cc`).
///
/// Перечислены не все страны, которые знает isdayoff.ru: у Турции, США и
/// Латвии данные заканчиваются на 2022 году, поэтому они не поддерживаются.
///
/// Перечисление помечено `#[non_exhaustive]`: добавление новой страны в
/// будущей версии не сломает чужой код с `match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Country {
    /// Россия (`ru`) — значение по умолчанию.
    #[default]
    Russia,
    /// Беларусь (`by`).
    Belarus,
    /// Казахстан (`kz`).
    Kazakhstan,
    /// Узбекистан (`uz`).
    Uzbekistan,
}

impl Country {
    /// Двухбуквенный код страны (ISO 3166-1 alpha-2), как его ждёт API.
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

/// Тип дня — один символ ответа API.
///
/// Варианты [`Shortened`](DayType::Shortened), [`Covid`](DayType::Covid) и
/// [`Holiday`](DayType::Holiday) приходят **только** если включён
/// соответствующий флаг в билдере (`pre`, `covid`, `holiday`); без флага
/// такие дни приходят как обычные `Workday`/`DayOff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DayType {
    /// `0` — рабочий день.
    Workday,
    /// `1` — выходной (нерабочий) день.
    DayOff,
    /// `2` — сокращённый предпраздничный рабочий день (флаг `pre`).
    Shortened,
    /// `4` — нерабочий день из-за COVID-19, только 2020 год (флаг `covid`).
    Covid,
    /// `8` — праздничный день (флаг `holiday`).
    Holiday,
}

impl DayType {
    /// Разбирает один символ ответа API.
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

    /// Нужно ли в этот день работать (рабочий или сокращённый день).
    pub fn is_working(self) -> bool {
        matches!(self, DayType::Workday | DayType::Shortened)
    }

    /// Нерабочий ли это день (выходной, праздник, «ковидный» день).
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

/// Конкретная дата и её тип.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Day {
    /// Дата.
    pub date: NaiveDate,
    /// Тип дня.
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
