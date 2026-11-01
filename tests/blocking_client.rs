//! Blocking client tests. The whole file compiles only with `--features blocking`.
#![cfg(feature = "blocking")]

mod common;

use common::{MockServer, Reply, dead_url, huge_body};
use isdayoff::blocking::IsDayOff;
use isdayoff::chrono::NaiveDate;
use isdayoff::{DayType, Error};
use std::time::Duration;

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

#[test]
fn timeout_is_reported() {
    let server = MockServer::start_with(vec![Reply::Hang]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .timeout(Duration::from_millis(200))
        .build_blocking()
        .unwrap();
    match client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()) {
        Err(Error::Http(e)) => assert!(e.is_timeout(), "{e:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn timeout_while_reading_body() {
    // Headers arrive, the body stalls: the timeout fires inside `Read`, which
    // reports it as `io::Error`; it must still come out as `Error::Http`.
    let server = MockServer::start_with(vec![Reply::Stall(
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n1".to_vec(),
    )]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .timeout(Duration::from_millis(300))
        .build_blocking()
        .unwrap();
    match client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()) {
        Err(Error::Http(e)) => assert!(e.is_timeout(), "{e:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn early_eof_in_body() {
    // Body shorter than Content-Length, then the connection closes.
    let server = MockServer::start_with(vec![Reply::Raw(
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n1".to_vec(),
    )]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .build_blocking()
        .unwrap();
    let res = client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert!(matches!(res, Err(Error::Http(_))), "{res:?}");
}

#[test]
fn oversized_body_is_not_buffered() {
    let server = MockServer::start_with(vec![huge_body(8 * 1024 * 1024)]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .build_blocking()
        .unwrap();
    assert!(matches!(
        client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        Err(Error::ResponseTooLarge { .. })
    ));
}

#[test]
fn connection_refused() {
    let client = IsDayOff::builder()
        .base_url(dead_url())
        .build_blocking()
        .unwrap();
    match client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()) {
        Err(Error::Http(e)) => assert!(e.is_connect(), "{e:?}"),
        other => panic!("{other:?}"),
    }
}
