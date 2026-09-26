//! IELTS practice app: vocabulary with FSRS reviews, and writing with versions.
//!
//! Layers (clean architecture), inner to outer:
//!   domain         data types and pure rules
//!   application    use cases + ports (traits for storage and scheduling)
//!   infrastructure SQLite and FSRS implementations of the ports
//!   presentation   HTTP handlers
//! `main` is the only place that knows every layer: it builds the concrete
//! pieces and plugs them together.

mod application;
mod config;
mod domain;
mod infrastructure;
mod presentation;

use std::io;
use std::sync::Arc;

use ntex::web::{self, middleware, types::JsonConfig};

use application::ai_feedback_service::AiFeedbackService;
use application::card_service::CardService;
use application::dashboard_service::DashboardService;
use application::mistake_service::MistakeService;
use application::ports::{CardRepository, MistakeRepository, WritingRepository, WritingReviewer};
use application::practice_service::PracticeService;
use application::review_service::ReviewService;
use application::writing_service::WritingService;
use config::Config;
use infrastructure::activity_repository::SqliteActivityRepository;
use infrastructure::ai_feedback_repository::SqliteAiFeedbackRepository;
use infrastructure::card_repository::SqliteCardRepository;
use infrastructure::chat_reviewer::ChatCompletionsReviewer;
use infrastructure::database;
use infrastructure::fsrs_scheduler::FsrsScheduler;
use infrastructure::mistake_repository::SqliteMistakeRepository;
use infrastructure::practice_repository::SqlitePracticeRepository;
use infrastructure::writing_repository::SqliteWritingRepository;
use presentation::AppState;

#[ntex::main]
async fn main() -> io::Result<()> {
    // In development, read settings from a .env file if there is one
    // (Docker passes them in through docker-compose instead).
    dotenvy::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::from_env().map_err(io::Error::other)?;
    let pool = database::connect(&config.database_path)
        .await
        .map_err(|err| io::Error::other(format!("database: {err}")))?;

    // Build the outer-layer implementations, then hand them to the services
    // as trait objects. Repositories used by several services are shared.
    let tz = config.tz;
    let card_repo: Arc<dyn CardRepository> = Arc::new(SqliteCardRepository::new(pool.clone()));
    let writing_repo: Arc<dyn WritingRepository> =
        Arc::new(SqliteWritingRepository::new(pool.clone()));
    let mistake_repo: Arc<dyn MistakeRepository> =
        Arc::new(SqliteMistakeRepository::new(pool.clone()));
    let scheduler = Arc::new(FsrsScheduler::new(config.desired_retention));

    // No API key = AI feedback off; the rest of the app works the same.
    let reviewer: Option<Arc<dyn WritingReviewer>> = match &config.ai.api_key {
        Some(key) => Some(Arc::new(
            ChatCompletionsReviewer::new(&config.ai.base_url, key.clone())
                .map_err(|err| io::Error::other(format!("AI client: {err}")))?,
        )),
        None => {
            log::warn!("AI_API_KEY not set: AI feedback is off");
            None
        }
    };

    let state = AppState {
        cards: CardService::new(card_repo.clone()),
        reviews: ReviewService::new(card_repo.clone(), scheduler, tz),
        writing: WritingService::new(writing_repo.clone(), tz),
        dashboard: DashboardService::new(
            card_repo,
            writing_repo.clone(),
            Arc::new(SqliteActivityRepository::new(pool.clone())),
            tz,
        ),
        practice: PracticeService::new(Arc::new(SqlitePracticeRepository::new(pool.clone())), tz),
        mistakes: MistakeService::new(mistake_repo.clone()),
        ai: AiFeedbackService::new(
            reviewer,
            Arc::new(SqliteAiFeedbackRepository::new(pool)),
            writing_repo,
            mistake_repo,
            config.ai.models,
            config.ai.default_model,
            tz,
        ),
    };

    log::info!(
        "listening on http://{} (db: {}, tz: {})",
        config.bind,
        config.database_path,
        config.tz
    );

    // Check once here, not in every worker.
    let static_dir = Some(config.static_dir).filter(|dir| std::path::Path::new(dir).is_dir());
    if static_dir.is_none() {
        log::warn!("static dir not found; serving the API only (use the Vite dev server)");
    }

    // ntex calls this closure once per worker thread to build that worker's app.
    web::server(async move || {
        web::App::new()
            .state(state.clone())
            // Default JSON limit is 32 KB; long essays with feedback need more.
            .state(JsonConfig::default().limit(1024 * 1024))
            .middleware(middleware::Logger::default())
            .configure(presentation::api_routes)
            .configure(|cfg| presentation::static_files(cfg, static_dir.as_deref()))
    })
    .bind(&config.bind)?
    // One user: two worker threads are plenty (the default is one per CPU core).
    .workers(2)
    .run()
    .await
}
