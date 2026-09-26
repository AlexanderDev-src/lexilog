//! Application layer: use cases ("add a card", "rate a review", ...).
//!
//! Each service coordinates domain rules and ports. It never touches SQL or
//! HTTP directly, so the same logic would work behind a CLI or in tests.

pub mod ai_feedback_service;
pub mod card_service;
pub mod dashboard_service;
pub mod mistake_service;
pub mod ports;
pub mod practice_service;
pub mod review_service;
pub mod writing_service;
