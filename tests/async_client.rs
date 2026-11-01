//! Async client tests against a local mock server: the whole chain
//! "builder → URL → HTTP → response parsing", without the internet.

mod common;

use common::{MockServer, Reply, dead_url, huge_body};
use isdayoff::chrono::{Month, NaiveDate};
use isdayoff::{Country, DayType, Error, IsDayOff};
use std::time::Duration;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[tokio::test]
async fn day_request_and_user_agent() {
    let server = MockServer::start(vec![(200, "8")]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .holiday(true)
        .user_agent_comment("tests")
        .build()
        .unwrap();

    let kind = client.day(date(2025, 1, 1)).await.unwrap();
    assert_eq!(kind, DayType::Holiday);

    let req = server.next_request();
    assert_eq!(
        req.request_line,
        "GET /api/getdata?year=2025&month=01&day=01&cc=ru&holiday=1 HTTP/1.1"
    );
    let ua = req.header("user-agent").unwrap();
    assert!(ua.starts_with("isdayoff-rust-lib/"), "{ua}");
    assert!(ua.ends_with("; tests)"), "{ua}");
}

#[tokio::test]
async fn month_maps_dates() {
    // February 2025 has 28 days
    let server = MockServer::start(vec![(200, "0000110000011000001100000110")]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .country(Country::Kazakhstan)
        .build()
        .unwrap();

    let days = client.month(2025, Month::February).await.unwrap();
    assert_eq!(days.len(), 28);
    assert_eq!(days[0].date, date(2025, 2, 1));
    assert_eq!(days[27].date, date(2025, 2, 28));
    assert_eq!(days[4].kind, DayType::DayOff);

    assert!(server.next_request().request_line.contains("cc=kz"));
}

#[tokio::test]
async fn api_error_codes() {
    let server = MockServer::start(vec![(400, "100"), (404, "101"), (200, "1")]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .build()
        .unwrap();

    let d = date(2025, 1, 1);
    assert!(matches!(client.day(d).await, Err(Error::BadRequest)));
    assert!(matches!(client.day(d).await, Err(Error::NotFound)));
    // A year expects 365 characters but only one arrived.
    assert!(matches!(
        client.year(2025).await,
        Err(Error::LengthMismatch {
            expected: 365,
            actual: 1
        })
    ));
}

#[tokio::test]
async fn reversed_range_fails_without_network() {
    // No server at all: the error must happen before any request.
    let client = IsDayOff::builder()
        .base_url("http://127.0.0.1:9/")
        .build()
        .unwrap();
    let err = client.range(date(2025, 2, 1), date(2025, 1, 1)).await;
    assert!(matches!(err, Err(Error::InvalidRange(_))));
}

#[test]
fn invalid_base_url_is_an_error_not_a_panic() {
    let err = IsDayOff::builder().base_url("not a url").build();
    assert!(matches!(err, Err(Error::InvalidBaseUrl(_))));
}

#[tokio::test]
async fn base_url_without_trailing_slash_keeps_prefix() {
    let server = MockServer::start(vec![(200, "1")]);
    // "http://127.0.0.1:port/" + "proxy" — no trailing slash.
    let client = IsDayOff::builder()
        .base_url(format!("{}proxy", server.base_url))
        .build()
        .unwrap();
    client.day(date(2025, 1, 1)).await.unwrap();
    assert!(
        server
            .next_request()
            .request_line
            .starts_with("GET /proxy/api/getdata?")
    );
}

#[tokio::test]
async fn timeout_is_reported() {
    let server = MockServer::start_with(vec![Reply::Hang]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .timeout(Duration::from_millis(200))
        .build()
        .unwrap();
    match client.day(date(2025, 1, 1)).await {
        Err(Error::Http(e)) => assert!(e.is_timeout(), "{e:?}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn timeout_while_reading_body() {
    let server = MockServer::start_with(vec![Reply::Stall(
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n1".to_vec(),
    )]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .timeout(Duration::from_millis(300))
        .build()
        .unwrap();
    match client.day(date(2025, 1, 1)).await {
        Err(Error::Http(e)) => assert!(e.is_timeout(), "{e:?}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn connection_refused() {
    let client = IsDayOff::builder().base_url(dead_url()).build().unwrap();
    match client.day(date(2025, 1, 1)).await {
        Err(Error::Http(e)) => assert!(e.is_connect(), "{e:?}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn connection_closed_without_response() {
    let server = MockServer::start_with(vec![Reply::Close]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .build()
        .unwrap();
    assert!(matches!(
        client.day(date(2025, 1, 1)).await,
        Err(Error::Http(_))
    ));
}

#[tokio::test]
async fn malformed_responses() {
    let server = MockServer::start_with(vec![
        Reply::Raw(b"HTTP/1.1 200 OK\r\nContent-Length: abc\r\n\r\n1".to_vec()),
        Reply::Raw(b"HTTP/1.1 200 OK\r\nBad Header Without Colon\r\n\r\n1".to_vec()),
        Reply::Raw(b"definitely not http\r\n\r\n".to_vec()),
        // Body shorter than Content-Length, then the connection closes.
        Reply::Raw(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n1".to_vec()),
    ]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .build()
        .unwrap();
    for i in 0..4 {
        let res = client.day(date(2025, 1, 1)).await;
        assert!(matches!(res, Err(Error::Http(_))), "reply #{i}: {res:?}");
    }
}

#[tokio::test]
async fn oversized_body_is_not_buffered() {
    let server = MockServer::start_with(vec![huge_body(8 * 1024 * 1024)]);
    let client = IsDayOff::builder()
        .base_url(&server.base_url)
        .build()
        .unwrap();
    assert!(matches!(
        client.day(date(2025, 1, 1)).await,
        Err(Error::ResponseTooLarge { .. })
    ));
}
