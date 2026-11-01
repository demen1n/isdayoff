//! Тесты против настоящего isdayoff.ru. По умолчанию пропускаются (`#[ignore]`),
//! чтобы `cargo test` не зависел от сети и не нагружал чужой сервис.
//!
//! Запуск: `cargo test --test live -- --ignored`

use isdayoff::chrono::{Month, NaiveDate};
use isdayoff::{DayType, Error, IsDayOff};

fn client() -> IsDayOff {
    IsDayOff::builder()
        .pre(true)
        .holiday(true)
        .user_agent_comment("integration tests")
        .build()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires network"]
async fn new_year_is_holiday() {
    let kind = client()
        .day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        .await
        .unwrap();
    assert_eq!(kind, DayType::Holiday);
}

#[tokio::test]
#[ignore = "requires network"]
async fn april_30_2025_is_shortened() {
    let days = client().month(2025, Month::April).await.unwrap();
    assert_eq!(days.len(), 30);
    // 30 апреля 2025 — сокращённый предпраздничный день.
    assert_eq!(days[29].kind, DayType::Shortened);
}

#[tokio::test]
#[ignore = "requires network"]
async fn ancient_year_has_no_data() {
    assert!(matches!(client().year(1990).await, Err(Error::NotFound)));
}
