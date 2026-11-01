//! Единый тип ошибки крейта.

/// Ошибки, которые возвращает библиотека.
///
/// `#[non_exhaustive]` позволяет добавлять новые варианты в минорных
/// версиях, не ломая пользователей: в их `match` обязателен `_ =>`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Сетевая ошибка, таймаут, ошибка TLS и т.п.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Адрес, переданный в [`IsDayOffBuilder::base_url`](crate::IsDayOffBuilder::base_url),
    /// не является корректным URL.
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),

    /// Код `100`: API считает дату или страну неверной.
    #[error("bad date or country (API returned 100)")]
    BadRequest,

    /// Код `101`: у сервиса нет данных за этот период.
    #[error("no data for this period (API returned 101)")]
    NotFound,

    /// Код `199`: внутренняя ошибка сервиса.
    #[error("isdayoff service error (API returned 199)")]
    ServiceError,

    /// Начало диапазона позже его конца, или дата вне диапазона `chrono`.
    #[error("invalid date range: {0}")]
    InvalidRange(&'static str),

    /// Ответ пришёл, но не похож ни на данные, ни на известный код ошибки.
    #[error("unexpected response (HTTP {status}): {body:?}")]
    UnexpectedResponse {
        /// HTTP-статус ответа.
        status: u16,
        /// Тело ответа (обрезанное до разумной длины).
        body: String,
    },

    /// В ответе встретился неизвестный символ.
    #[error("unknown day type code: {0:?}")]
    UnknownDayType(char),

    /// Количество дней в ответе не совпало с запрошенным периодом.
    #[error("expected {expected} days in response, got {actual}")]
    LengthMismatch {
        /// Сколько дней ожидалось.
        expected: usize,
        /// Сколько символов пришло.
        actual: usize,
    },
}
