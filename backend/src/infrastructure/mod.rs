//! Infrastructure layer: real implementations of the ports
//! (SQLite through sqlx, spaced repetition through the `fsrs` crate,
//! AI feedback through an OpenAI-compatible HTTP API).

pub mod activity_repository;
pub mod ai_feedback_repository;
pub mod card_repository;
pub mod chat_reviewer;
pub mod database;
pub mod fsrs_scheduler;
pub mod mistake_repository;
pub mod practice_repository;
pub mod writing_repository;
