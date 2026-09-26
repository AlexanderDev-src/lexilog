use ntex::web::HttpResponse;
use ntex::web::types::State;

use super::AppState;
use crate::domain::error::AppError;

/// GET /api/dashboard
pub async fn dashboard(state: State<AppState>) -> Result<HttpResponse, AppError> {
    let dashboard = state.dashboard.dashboard().await?;
    Ok(HttpResponse::Ok().json(&dashboard))
}
