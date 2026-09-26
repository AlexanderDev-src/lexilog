use ntex::http::StatusCode;
use ntex::web::{HttpRequest, HttpResponse, WebResponseError};

use crate::domain::error::AppError;

/// Tells ntex how to turn an `AppError` into an HTTP response. Because of
/// this, handlers can return `Result<HttpResponse, AppError>` and use `?`.
///
/// Body is always JSON: `{"error": "card not found"}`.
impl WebResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self, _: &HttpRequest) -> HttpResponse {
        let message = match self {
            // Details are already in the server log; keep the response short.
            AppError::Internal(_) => "internal error".to_string(),
            other => other.to_string(),
        };
        HttpResponse::build(self.status_code()).json(&serde_json::json!({ "error": message }))
    }
}
