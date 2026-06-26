use thiserror::Error;

/// `check-for-updates` slice 内部のエラー (`workflows/check-for-updates.md#errors`)。
///
/// application service の outer layer で握り潰し、UI 層には伝搬しない (S14 silent failure)。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UpdateError {
    #[error("network error while fetching latest release")]
    NetworkError,
    #[error("failed to parse release version string")]
    ParseError,
    #[error("rate limited by upstream API")]
    RateLimited,
}
