use ntex::web::HttpResponse;
use ntex::web::types::{Json, Path, Query, State};

use super::AppState;
use crate::domain::card::{CardFilter, CardInput};
use crate::domain::error::AppError;

// Each argument is an "extractor": ntex fills it from the request before
// calling the function. State = shared app state, Path = `{id}` in the URL,
// Query = `?q=...`, Json = the request body.

/// GET /api/cards?q=&tag=&limit=&offset=
pub async fn list(
    state: State<AppState>,
    filter: Query<CardFilter>,
) -> Result<HttpResponse, AppError> {
    let cards = state.cards.list(&filter).await?;
    Ok(HttpResponse::Ok().json(&cards))
}

/// GET /api/cards/{id}
pub async fn get(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    let card = state.cards.get(id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(&card))
}

/// POST /api/cards
pub async fn create(
    state: State<AppState>,
    body: Json<CardInput>,
) -> Result<HttpResponse, AppError> {
    let card = state.cards.create(body.into_inner()).await?;
    Ok(HttpResponse::Created().json(&card))
}

/// PUT /api/cards/{id}
pub async fn update(
    state: State<AppState>,
    id: Path<i64>,
    body: Json<CardInput>,
) -> Result<HttpResponse, AppError> {
    let card = state
        .cards
        .update(id.into_inner(), body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(&card))
}

/// DELETE /api/cards/{id}
pub async fn delete(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    state.cards.delete(id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// GET /api/tags
pub async fn tags(state: State<AppState>) -> Result<HttpResponse, AppError> {
    let tags = state.cards.tags().await?;
    Ok(HttpResponse::Ok().json(&tags))
}
