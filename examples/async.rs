//! Запуск: `cargo run --example async`

use isdayoff::chrono::{Datelike, Local, Month};
use isdayoff::{DayType, IsDayOff};

#[tokio::main]
async fn main() -> Result<(), isdayoff::Error> {
    let client = IsDayOff::builder()
        .pre(true)
        .holiday(true)
        .user_agent_comment("example")
        .build()?;

    let today = Local::now().date_naive();
    println!("Сегодня {today}: {}", client.day(today).await?);

    let month = Month::try_from(today.month() as u8).expect("month is always 1..=12");
    let days = client.month(today.year(), month).await?;
    let working = days.iter().filter(|d| d.kind.is_working()).count();
    let shortened = days.iter().filter(|d| d.kind == DayType::Shortened).count();
    println!(
        "В этом месяце {working} рабочих дней из {}, сокращённых: {shortened}",
        days.len()
    );
    Ok(())
}
