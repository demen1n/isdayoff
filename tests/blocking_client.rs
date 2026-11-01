//! Blocking client tests. The whole file compiles only with `--features blocking`.
#![cfg(feature = "blocking")]

mod common;

use common::MockServer;
use isdayoff::blocking::IsDayOff;
use isdayoff::chrono::NaiveDate;
use isdayoff::{DayType, Error};

#[test]
fn range_request() {
    let server = MockServer::start(vec![(200, "0112"), (500, "199")]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .pre(true)
        .build_blocking()
        .unwrap();

    let from = NaiveDate::from_ymd_opt(2025, 12, 29).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let days = client.range(from, to).unwrap();
    let kinds: Vec<_> = days.iter().map(|d| d.kind).collect();
    assert_eq!(
        kinds,
        [
            DayType::Workday,
            DayType::DayOff,
            DayType::DayOff,
            DayType::Shortened
        ]
    );
    assert_eq!(days[3].date, to);
    assert_eq!(
        server.next_request().request_line,
        "GET /api/getdata?date1=20251229&date2=20260101&cc=ru&pre=1 HTTP/1.1"
    );

    assert!(matches!(client.day(from), Err(Error::ServiceError)));
}
