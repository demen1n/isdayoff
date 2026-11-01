//! Запуск: `cargo run --example blocking --features blocking`

use isdayoff::blocking::IsDayOff;
use isdayoff::chrono::{Days, Local};

fn main() -> Result<(), isdayoff::Error> {
    let client = IsDayOff::builder()
        .holiday(true)
        .user_agent_comment("example")
        .build_blocking()?;

    let today = Local::now().date_naive();
    let week = today.checked_add_days(Days::new(6)).expect("date in range");
    for day in client.range(today, week)? {
        println!("{} ({}) — {}", day.date, day.date.format("%a"), day.kind);
    }
    Ok(())
}
