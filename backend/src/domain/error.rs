use thiserror::Error;

/// Every error the app can return.
///
/// `#[derive(Error)]` (from the `thiserror` crate) writes the `Display`
/// implementation for us from the `#[error("...")]` attributes.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0} not found")]
    NotFound(&'static str),

    #[error("{0}")]
    Validation(String),

    /// A daily quota is used up (the AI gateway's token limit).
    #[error("{0}")]
    RateLimited(String),

    /// An outside service is missing or not answering (AI not configured,
    /// gateway down, key rejected).
    #[error("{0}")]
    Unavailable(String),

    /// Database or other unexpected failure. The message is logged, not shown
    /// to the user in detail.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Shorthand so functions can write `AppResult<Card>`
/// instead of `Result<Card, AppError>`.
pub type AppResult<T> = Result<T, AppError>;
