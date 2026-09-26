//! Presentation layer: HTTP. Turns requests into service calls and results
//! into JSON. No business rules live here.

mod ai_handlers;
mod card_handlers;
mod dashboard_handlers;
mod error;
mod mistake_handlers;
mod practice_handlers;
mod review_handlers;
mod writing_handlers;

use ntex::web::{self, HttpResponse};

use crate::application::ai_feedback_service::AiFeedbackService;
use crate::application::card_service::CardService;
use crate::application::dashboard_service::DashboardService;
use crate::application::mistake_service::MistakeService;
use crate::application::practice_service::PracticeService;
use crate::application::review_service::ReviewService;
use crate::application::writing_service::WritingService;

/// Everything the handlers need. Each ntex worker thread gets its own clone;
/// the clones are cheap because the services only hold `Arc`s.
#[derive(Clone)]
pub struct AppState {
    pub cards: CardService,
    pub reviews: ReviewService,
    pub writing: WritingService,
    pub dashboard: DashboardService,
    pub practice: PracticeService,
    pub mistakes: MistakeService,
    pub ai: AiFeedbackService,
}

/// All JSON endpoints, under `/api`.
pub fn api_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(web::resource("/health").route(web::get().to(health)))
            // Vocabulary
            .service(
                web::resource("/cards")
                    .route(web::get().to(card_handlers::list))
                    .route(web::post().to(card_handlers::create)),
            )
            .service(
                web::resource("/cards/{id}")
                    .route(web::get().to(card_handlers::get))
                    .route(web::put().to(card_handlers::update))
                    .route(web::delete().to(card_handlers::delete)),
            )
            .service(web::resource("/tags").route(web::get().to(card_handlers::tags)))
            // Reviews
            .service(web::resource("/review/due").route(web::get().to(review_handlers::due)))
            .service(
                web::resource("/cards/{id}/review").route(web::post().to(review_handlers::review)),
            )
            // Writing
            .service(
                web::resource("/pieces")
                    .route(web::get().to(writing_handlers::list))
                    .route(web::post().to(writing_handlers::create)),
            )
            .service(
                web::resource("/pieces/{id}")
                    .route(web::get().to(writing_handlers::get))
                    .route(web::put().to(writing_handlers::update))
                    .route(web::delete().to(writing_handlers::delete)),
            )
            .service(
                web::resource("/pieces/{id}/versions")
                    .route(web::post().to(writing_handlers::add_version)),
            )
            .service(
                web::resource("/versions/{id}")
                    .route(web::put().to(writing_handlers::update_version))
                    .route(web::delete().to(writing_handlers::delete_version)),
            )
            // Dashboard and practice log
            .service(
                web::resource("/dashboard").route(web::get().to(dashboard_handlers::dashboard)),
            )
            .service(
                web::resource("/practice")
                    .route(web::get().to(practice_handlers::list))
                    .route(web::post().to(practice_handlers::create)),
            )
            .service(
                web::resource("/practice/{id}")
                    .route(web::put().to(practice_handlers::update))
                    .route(web::delete().to(practice_handlers::delete)),
            )
            // Mistake log
            .service(
                web::resource("/pieces/{id}/mistakes")
                    .route(web::get().to(mistake_handlers::for_piece))
                    .route(web::put().to(mistake_handlers::replace)),
            )
            .service(web::resource("/mistake-tags").route(web::get().to(mistake_handlers::tags)))
            .service(web::resource("/mistakes/trend").route(web::get().to(mistake_handlers::trend)))
            // AI feedback
            .service(web::resource("/ai/status").route(web::get().to(ai_handlers::status)))
            .service(
                web::resource("/pieces/{id}/ai-feedback")
                    .route(web::get().to(ai_handlers::for_piece)),
            )
            .service(
                web::resource("/versions/{id}/ai-feedback")
                    .route(web::post().to(ai_handlers::review)),
            ),
    );
}

/// Serves the built Svelte app (index.html, JS, CSS).
/// `None` in development, where the Vite dev server does this instead.
pub fn static_files(cfg: &mut web::ServiceConfig, dir: Option<&str>) {
    if let Some(dir) = dir {
        cfg.service(ntex_files::Files::new("/", dir).index_file("index.html"));
    }
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(&serde_json::json!({ "status": "ok" }))
}
