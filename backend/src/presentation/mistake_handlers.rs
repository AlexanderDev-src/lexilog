use ntex::web::HttpResponse;
use ntex::web::types::{Json, Path, Query, State};
use serde::Deserialize;

use super::AppState;
use crate::domain::error::AppError;
use crate::domain::mistakes::{MistakeCount, TrendBucket};

/// GET /api/pieces/{id}/mistakes
pub async fn for_piece(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    let mistakes = state.mistakes.for_piece(id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(&mistakes))
}

/// PUT /api/pieces/{id}/mistakes  body: [{"tag": "...", "count": 2}, ...]
pub async fn replace(
    state: State<AppState>,
    id: Path<i64>,
    body: Json<Vec<MistakeCount>>,
) -> Result<HttpResponse, AppError> {
    let mistakes = state
        .mistakes
        .replace(id.into_inner(), body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(&mistakes))
}

/// GET /api/mistake-tags
pub async fn tags(state: State<AppState>) -> Result<HttpResponse, AppError> {
    let tags = state.mistakes.tags().await?;
    Ok(HttpResponse::Ok().json(&tags))
}

#[derive(Deserialize)]
pub struct TrendQuery {
    #[serde(default)]
    bucket: TrendBucket,
}

/// GET /api/mistakes/trend?bucket=month|week
pub async fn trend(
    state: State<AppState>,
    query: Query<TrendQuery>,
) -> Result<HttpResponse, AppError> {
    let trend = state.mistakes.trend(query.bucket).await?;
    Ok(HttpResponse::Ok().json(&trend))
}
