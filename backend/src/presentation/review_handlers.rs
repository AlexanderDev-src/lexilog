use ntex::web::HttpResponse;
use ntex::web::types::{Json, Path, Query, State};
use serde::Deserialize;

use super::AppState;
use crate::domain::error::AppError;
use crate::domain::review::Rating;

#[derive(Deserialize)]
pub struct DueQuery {
    limit: Option<i64>,
}

/// GET /api/review/due?limit=100
pub async fn due(state: State<AppState>, query: Query<DueQuery>) -> Result<HttpResponse, AppError> {
    let queue = state.reviews.due(query.limit.unwrap_or(100)).await?;
    Ok(HttpResponse::Ok().json(&queue))
}

#[derive(Deserialize)]
pub struct ReviewBody {
    /// 1 = Again, 2 = Hard, 3 = Good, 4 = Easy. Other numbers are rejected
    /// while parsing the JSON (see `Rating` in the domain).
    rating: Rating,
    duration_ms: Option<i64>,
}

/// POST /api/cards/{id}/review
pub async fn review(
    state: State<AppState>,
    id: Path<i64>,
    body: Json<ReviewBody>,
) -> Result<HttpResponse, AppError> {
    let card = state
        .reviews
        .review(id.into_inner(), body.rating, body.duration_ms)
        .await?;
    Ok(HttpResponse::Ok().json(&card))
}
