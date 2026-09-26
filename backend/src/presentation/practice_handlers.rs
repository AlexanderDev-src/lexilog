use ntex::web::HttpResponse;
use ntex::web::types::{Json, Path, Query, State};
use serde::Deserialize;

use super::AppState;
use crate::domain::error::AppError;
use crate::domain::practice::PracticeInput;

#[derive(Deserialize)]
pub struct ListQuery {
    limit: Option<i64>,
}

/// GET /api/practice?limit=50
pub async fn list(
    state: State<AppState>,
    query: Query<ListQuery>,
) -> Result<HttpResponse, AppError> {
    let sessions = state.practice.list(query.limit.unwrap_or(50)).await?;
    Ok(HttpResponse::Ok().json(&sessions))
}

/// POST /api/practice
pub async fn create(
    state: State<AppState>,
    body: Json<PracticeInput>,
) -> Result<HttpResponse, AppError> {
    let session = state.practice.create(body.into_inner()).await?;
    Ok(HttpResponse::Created().json(&session))
}

/// PUT /api/practice/{id}
pub async fn update(
    state: State<AppState>,
    id: Path<i64>,
    body: Json<PracticeInput>,
) -> Result<HttpResponse, AppError> {
    let session = state
        .practice
        .update(id.into_inner(), body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(&session))
}

/// DELETE /api/practice/{id}
pub async fn delete(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    state.practice.delete(id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
