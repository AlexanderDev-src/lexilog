use ntex::web::HttpResponse;
use ntex::web::types::{Json, Path, State};
use serde::Deserialize;

use super::AppState;
use crate::domain::error::AppError;

/// GET /api/ai/status
pub async fn status(state: State<AppState>) -> Result<HttpResponse, AppError> {
    let status = state.ai.status().await?;
    Ok(HttpResponse::Ok().json(&status))
}

/// GET /api/pieces/{id}/ai-feedback
pub async fn for_piece(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    let records = state.ai.for_piece(id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(&records))
}

#[derive(Deserialize)]
pub struct ReviewBody {
    /// Leave out to use AI_DEFAULT_MODEL.
    #[serde(default)]
    model: Option<String>,
}

/// POST /api/versions/{id}/ai-feedback  body: {"model": "claude-sonnet-5"} (optional)
pub async fn review(
    state: State<AppState>,
    id: Path<i64>,
    body: Json<ReviewBody>,
) -> Result<HttpResponse, AppError> {
    let record = state
        .ai
        .review_version(id.into_inner(), body.into_inner().model)
        .await?;
    Ok(HttpResponse::Created().json(&record))
}
