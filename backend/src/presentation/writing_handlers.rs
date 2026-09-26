use ntex::http::header;
use ntex::util::Bytes;
use ntex::web::HttpResponse;
use ntex::web::types::{Json, Path, State};

use super::AppState;
use crate::domain::error::AppError;
use crate::domain::writing::{NewPiece, PieceUpdate, VersionUpdate};

/// GET /api/pieces
pub async fn list(state: State<AppState>) -> Result<HttpResponse, AppError> {
    let pieces = state.writing.list().await?;
    Ok(HttpResponse::Ok().json(&pieces))
}

/// GET /api/pieces/{id}
pub async fn get(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    let piece = state.writing.get(id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(&piece))
}

/// POST /api/pieces
pub async fn create(
    state: State<AppState>,
    body: Json<NewPiece>,
) -> Result<HttpResponse, AppError> {
    let piece = state.writing.create(body.into_inner()).await?;
    Ok(HttpResponse::Created().json(&piece))
}

/// PUT /api/pieces/{id}
pub async fn update(
    state: State<AppState>,
    id: Path<i64>,
    body: Json<PieceUpdate>,
) -> Result<HttpResponse, AppError> {
    let piece = state
        .writing
        .update(id.into_inner(), body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(&piece))
}

/// DELETE /api/pieces/{id}
pub async fn delete(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    state.writing.delete(id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// POST /api/pieces/{id}/versions
pub async fn add_version(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    let version = state.writing.add_version(id.into_inner()).await?;
    Ok(HttpResponse::Created().json(&version))
}

/// PUT /api/versions/{id}
pub async fn update_version(
    state: State<AppState>,
    id: Path<i64>,
    body: Json<VersionUpdate>,
) -> Result<HttpResponse, AppError> {
    let version = state
        .writing
        .update_version(id.into_inner(), body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(&version))
}

/// DELETE /api/versions/{id}
pub async fn delete_version(
    state: State<AppState>,
    id: Path<i64>,
) -> Result<HttpResponse, AppError> {
    state.writing.delete_version(id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// PUT /api/pieces/{id}/image  body: the PNG or WebP bytes
pub async fn put_image(
    state: State<AppState>,
    id: Path<i64>,
    body: Bytes,
) -> Result<HttpResponse, AppError> {
    let piece = state.writing.set_image(id.into_inner(), &body).await?;
    Ok(HttpResponse::Ok().json(&piece))
}

/// GET /api/pieces/{id}/image
pub async fn get_image(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    let image = state.writing.image(id.into_inner()).await?;
    Ok(HttpResponse::Ok()
        .content_type(image.mime)
        // The editor adds ?v=<upload time> to the URL, so a replaced image
        // gets a new URL; still, ask the browser to check every time.
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(image.data))
}

/// DELETE /api/pieces/{id}/image
pub async fn delete_image(state: State<AppState>, id: Path<i64>) -> Result<HttpResponse, AppError> {
    state.writing.delete_image(id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
