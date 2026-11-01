//! The crate-wide error type.

/// Errors returned by this library.
///
/// `#[non_exhaustive]` lets new variants be added in minor releases without
/// breaking users: their `match` must include a `_ =>` arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Network error, timeout, TLS failure, etc.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Reading the response body failed with an I/O error not coming from
    /// the HTTP client itself (blocking client only).
    #[error("failed to read response body: {0}")]
    Io(#[source] std::io::Error),

    /// The address passed to [`IsDayOffBuilder::base_url`](crate::IsDayOffBuilder::base_url)
    /// is not a valid URL.
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),

    /// Code `100`: the API considers the date or country invalid.
    #[error("bad date or country (API returned 100)")]
    BadRequest,

    /// Code `101`: the service has no data for this period.
    #[error("no data for this period (API returned 101)")]
    NotFound,

    /// Code `199`: internal service error.
    #[error("isdayoff service error (API returned 199)")]
    ServiceError,

    /// The range start is after its end, or a date is outside `chrono`'s range.
    #[error("invalid date range: {0}")]
    InvalidRange(&'static str),

    /// A response arrived but is neither data nor a known error code.
    #[error("unexpected response (HTTP {status}): {body:?}")]
    UnexpectedResponse {
        /// HTTP status of the response.
        status: u16,
        /// Response body (truncated to a reasonable length).
        body: String,
    },

    /// A successful response is larger than the requested period could
    /// possibly need, so reading stopped early. Protects against unbounded
    /// memory use with a misbehaving server or proxy.
    #[error("response body exceeds {limit} bytes")]
    ResponseTooLarge {
        /// Maximum accepted body size in bytes.
        limit: usize,
    },

    /// The response contains an unknown character.
    #[error("unknown day type code: {0:?}")]
    UnknownDayType(char),

    /// The number of days in the response doesn't match the requested period.
    #[error("expected {expected} days in response, got {actual}")]
    LengthMismatch {
        /// Expected number of days.
        expected: usize,
        /// Number of characters received.
        actual: usize,
    },
}
