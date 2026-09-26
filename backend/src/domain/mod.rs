//! Domain layer: the app's data types and pure rules.
//!
//! Nothing in here knows about HTTP, SQL or the FSRS crate. Code in the
//! other layers depends on this module, never the other way round.

pub mod activity;
pub mod calendar;
pub mod card;
pub mod error;
pub mod feedback;
pub mod mistakes;
pub mod practice;
pub mod review;
pub mod writing;
