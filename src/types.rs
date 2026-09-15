use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("bad date or country (API returned 100)")]
    BadDate,

    #[error("no data for this period (API returned 101)")]
    NotFound,

    #[error("isdayoff service error (API returned 199)")]
    ServiceError,

    #[error("unknown day type: {0}")]
    UnknownDayType(u8),
}

// страны, поддерживаемые API isdayoff.ru
// указаны не все, так как Турция, США, Латвия имеют данные до 2022 года
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Country {
    #[default]
    Russia,
    Belarus,
    Kazakhstan,
    Uzbekistan
}

impl Country {
    pub fn as_str(self) -> &'static str {
        match self {
            Country::Russia => "ru",
            Country::Belarus => "by",
            Country::Kazakhstan => "kz",
            Country::Uzbekistan => "uz",
        }
    }
}

impl fmt::Display for Country {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DayType {
    Workday,
    Dayoff,
    Shortened,
    Covid,
    Holiday,
}

impl DayType {
    pub fn from_char(c: char) -> Result<DayType, Error> {
        match c {
            '0' => Ok(DayType::Workday),
            '1' => Ok(DayType::Dayoff),
            '2' => Ok(DayType::Shortened),
            '4' => Ok(DayType::Covid),
            '8' => Ok(DayType::Holiday),
            _  => Err(Error::UnknownDayType(c as u8)),
        }
    }
}

impl fmt::Display for DayType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DayType::Workday => write!(f, "Workday"),
            DayType::Dayoff => write!(f, "Day off"),
            DayType::Shortened => write!(f, "Shortened day"),
            DayType::Covid => write!(f, "Covid day"),
            DayType::Holiday => write!(f, "Holiday"),
        }
    }
}