# План реализации: isdayoff-rs

## Что делает API

`https://isdayoff.ru/api/getdata` — возвращает текстовые коды.

Запрос дат — один из вариантов:

| Параметры | Ответ |
|---|---|
| `year=2025&month=01&day=01` | `1` — один символ |
| `year=2025&month=01` | `1111111100110...` — строка за месяц |
| `year=2025` | `1111111100...` — строка за год (365/366 символов) |
| `date1=20250101&date2=20250201` | строка за произвольный диапазон дат |

Символы ответа:

| Код | Значение | Условие появления |
|---|---|---|
| `0` | рабочий день | всегда |
| `1` | выходной/нерабочий день | всегда |
| `2` | сокращённый (предпраздничный) рабочий день | только если `pre=1` |
| `4` | рабочий день, но с ограничениями (пандемия) | только если `covid=1` |
| `8` | праздничный день (выделен отдельно от простого выходного) | только если `holiday=1` |

Коды ошибок (вместо тела с датами):

| Код | Значение |
|---|---|
| `100` | неверная дата или страна |
| `101` | нет данных на этот период |
| `199` | внутренняя ошибка сервиса |

Параметры запроса:

- `cc` — двухбуквенный код страны, ISO 3166-1 alpha-2 (**не `country`** — это была ошибка в первой версии плана). По умолчанию `ru`.
- `pre` — `1`, чтобы помечать сокращённые дни кодом `2`.
- `sd` — `1`, чтобы считать неделю шестидневной.
- `covid` — `1`, чтобы помечать «ковидные» рабочие дни кодом `4`.
- `holiday` — `1`, чтобы отдельно помечать праздники кодом `8` (без этого праздники приходят как обычный `1`).
- `delimeter` — разделитель между днями в ответе за диапазон (по умолчанию нет, можно `%0A` и т.д.).
- `tz` — часовой пояс в формате IANA.

Поддерживаемые страны (`cc`): `ru` (Россия), `by` (Беларусь), `kz` (Казахстан), `us` (США), `uz` (Узбекистан), `tr` (Турция), `lv` (Латвия). Украины (`ua`) в списке нет — в первой версии плана это было ошибкой.

**Важный вывод для дизайна:** коды `2`/`4`/`8` физически не могут появиться в ответе, если клиент не передал соответствующий флаг (`pre`/`covid`/`holiday`). Поэтому `DayType` с 4+ вариантами имеет смысл, только если билдер реально даёт включить эти флаги — иначе `Shortened`/`Holiday` будут недостижимым кодом.

---

## Структура файлов

```
src/
├── lib.rs          — точка входа: pub mod + pub use
├── types.rs        — все типы: DayType, Country, Error
├── client.rs       — структура IsDayOff, builder, построение URL
└── blocking.rs     — blocking-версия (за feature-флагом)
```

Async-методы живут прямо в `client.rs`, blocking — в `blocking.rs` (отдельный impl-блок, чтобы не мешать).

---

## Зависимости (Cargo.toml)

```toml
[features]
default = ["async"]
async    = ["reqwest/default-tls"]
blocking = ["reqwest/blocking"]

[dependencies]
reqwest   = { version = "0.13", default-features = false, optional = true }
thiserror = "2"
chrono    = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

> **Что поменялось относительно первой версии:**
> - `serde` убран — API отдаёт голый текст, а не JSON, сериализация типам крейта не нужна. Если понадобится потребителям — добавим отдельно, когда появится реальный кейс.
> - `tokio` переехал в `dev-dependencies`. Библиотечному `async fn` рантайм для *определения* не нужен — он нужен только вызывающему коду (нашим тестам/примерам). Раз `blocking.rs` использует отдельный `reqwest::blocking::Client`, а не оборачивает async через `block_on`, тянуть `tokio` в обычные зависимости незачем.
> - Версия `reqwest` поднята до `0.13` — в `Cargo.toml` уже стоит она, план был написан под `0.12`.
> - `chrono` без фичи `serde` — она не используется, пока `serde` не в зависимостях.

> **Почему два feature-флага (`async`/`blocking`)?**
> `reqwest` умеет работать в двух режимах: обычный async (`reqwest::Client`) и blocking (`reqwest::blocking::Client`).
> Позволяем пользователю выбрать, что нужно, чтобы не тянуть лишние зависимости.

---

## Шаг 1 — `src/types.rs`

Текущий код уже частично реализован — вот что нужно поправить/дополнить:

```rust
// Страны, которые поддерживает API (параметр cc)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    Russia,      // "ru"
    Belarus,     // "by"
    Kazakhstan,  // "kz"
    Usa,         // "us"
    Uzbekistan,  // "uz"
    Turkey,      // "tr"
    Latvia,      // "lv"
}

impl Country {
    pub fn as_str(self) -> &'static str { ... }
}
```

```rust
// Тип ошибки — thiserror генерирует impl std::error::Error автоматически
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("bad date or country (API returned 100)")]
    BadDate,

    #[error("no data for this period (API returned 101)")]
    NotFound,

    #[error("isdayoff service error (API returned 199)")]
    ServiceError,

    #[error("unknown day type code: {0:?}")]
    UnknownDayType(char),
}
```

```rust
// DayType и парсинг символа ответа
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DayType {
    Workday,          // '0'
    Dayoff,           // '1'
    Shortened,        // '2', только при pre=1
    WorkdayRestricted,// '4', только при covid=1
    Holiday,          // '8', только при holiday=1
}

impl DayType {
    pub fn from_char(c: char) -> Result<Self, Error> { ... }
}
```

**Что убрать:** `impl TryFrom<u8> for DayType` — дублирует `from_char` (парсим строку ответа посимвольно как `char`, отдельный путь через `u8` не нужен — лишняя абстракция под несуществующий кейс).

---

## Шаг 2 — `src/client.rs`

```rust
pub struct IsDayOff {
    base_url: String,   // для тестов можно подменить
    country: Country,
    pre: bool,          // -> query cc + pre
    sd: bool,           // -> query sd
    covid: bool,        // -> query covid
    holiday: bool,       // -> query holiday
    http: reqwest::Client,
}

// Builder для удобной конфигурации
pub struct IsDayOffBuilder { ... }

impl IsDayOff {
    pub fn builder() -> IsDayOffBuilder { ... }

    // приватный хелпер — строит URL из параметров (cc, pre, sd, covid, holiday, year/month/day)
    fn build_url(&self, year: i32, month: Option<u32>, day: Option<u32>) -> String { ... }

    // приватный хелпер — проверяет код ошибки (100/101/199), потом парсит тело в Vec<DayType>
    fn parse_response(body: &str) -> Result<Vec<DayType>, Error> { ... }
}
```

Пример использования, к которому идём:
```rust
let client = IsDayOff::builder()
    .country(Country::Russia)
    .pre(true)
    .holiday(true)
    .build();

// async
let day = client.day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()).await?;
let month = client.month(2025, 1).await?;
let year = client.year(2025).await?;

// blocking (feature = "blocking")
let day = client.blocking().day(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())?;
```

Флаги `pre`/`sd`/`covid`/`holiday` в билдере — по умолчанию `false` (как и в самом API), включать явно.

---

## Шаг 3 — async методы в `client.rs`

```rust
#[cfg(feature = "async")]
impl IsDayOff {
    pub async fn day(&self, date: chrono::NaiveDate) -> Result<DayType, Error> {
        let url = self.build_url(date.year(), Some(date.month()), Some(date.day()));
        let body = self.http.get(&url).send().await?.text().await?;
        let days = Self::parse_response(&body)?;
        Ok(days.into_iter().next().unwrap())
    }

    pub async fn month(&self, year: i32, month: u32) -> Result<Vec<DayType>, Error> { ... }
    pub async fn year(&self, year: i32) -> Result<Vec<DayType>, Error> { ... }
}
```

---

## Шаг 4 — `src/blocking.rs`

Обёртка, которая держит `reqwest::blocking::Client` и повторяет те же методы без `async/await`.

```rust
#[cfg(feature = "blocking")]
pub struct BlockingClient<'a> {
    inner: &'a IsDayOff,
    http: reqwest::blocking::Client,
}

#[cfg(feature = "blocking")]
impl IsDayOff {
    pub fn blocking(&self) -> BlockingClient<'_> { ... }
}

#[cfg(feature = "blocking")]
impl BlockingClient<'_> {
    pub fn day(&self, date: chrono::NaiveDate) -> Result<DayType, Error> { ... }
    pub fn month(&self, year: i32, month: u32) -> Result<Vec<DayType>, Error> { ... }
    pub fn year(&self, year: i32) -> Result<Vec<DayType>, Error> { ... }
}
```

---

## Шаг 5 — `src/lib.rs`

```rust
pub mod types;
pub mod client;

#[cfg(feature = "blocking")]
pub mod blocking;

// Вытаскиваем самое важное на верхний уровень
pub use types::{Country, DayType, Error};
pub use client::IsDayOff;
```

---

## Шаг 6 — тесты

- **Unit-тесты** прямо в `types.rs`: парсинг символов (`0`/`1`/`2`/`4`/`8`), обработка `100`/`101`/`199` — не требуют сети.
- **Integration-тесты** в `tests/integration.rs`: реальные запросы к API. Пометить `#[ignore]`, чтобы `cargo test` по умолчанию их не гонял без сети — запускать явно через `cargo test --test integration -- --ignored`.

---

## Порядок работы

1. `types.rs` — поправить `Country` (актуальный список стран), `Error` (добавить `ServiceError`, поправить `UnknownDayType`), убрать дублирующий `TryFrom<u8>`, расширить `DayType::from_char` на `4`/`8`
2. `Cargo.toml` — убрать `serde`, перенести `tokio` в `dev-dependencies`, добавить feature-флаги `async`/`blocking`
3. `client.rs` — структура `IsDayOff`, builder (`country`/`pre`/`sd`/`covid`/`holiday`), `build_url`, `parse_response`
4. `lib.rs` — создать, всё прокинуть
5. async методы в `client.rs`
6. `blocking.rs`
7. тесты (unit без сети + integration с `#[ignore]`)
