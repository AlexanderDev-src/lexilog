//! Ports: what the application layer needs from the outside world.
//!
//! These are traits (interfaces). The services below only talk to these
//! traits; the `infrastructure` layer provides the real SQLite/FSRS versions.
//! That keeps SQL and crate details out of the business logic, and lets a
//! test swap in a different implementation.
//!
//! About `#[async_trait]`: Rust can't yet put `async fn` in a trait that is
//! used as `dyn Trait` (a trait object). The `async_trait` macro rewrites each
//! `async fn` so it returns a boxed future, which makes it work.
//! `Send + Sync` means one shared instance can be used from any thread.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

use crate::domain::activity::DayActivity;
use crate::domain::card::{Card, CardFilter, CardInput, DeckWord, DueBreakdown, TagCount};
use crate::domain::error::AppResult;
use crate::domain::feedback::{AiCall, AiFeedbackRecord, ReviewOutcome, ReviewRequest};
use crate::domain::image::{ImageMeta, StoredImage};
use crate::domain::mistakes::{
    MistakeCount, MistakeTagSummary, PeriodCount, PeriodWords, TrendBucket,
};
use crate::domain::practice::{NewPractice, PracticeSession};
use crate::domain::review::{MemoryState, NewReviewLog, NextStates};
use crate::domain::writing::{
    Piece, PieceSummary, PieceUpdate, Version, VersionUpdate, WritingKind,
};

#[async_trait]
pub trait CardRepository: Send + Sync {
    async fn list(&self, filter: &CardFilter) -> AppResult<Vec<Card>>;
    async fn get(&self, id: i64) -> AppResult<Option<Card>>;
    async fn create(&self, input: &CardInput, now: DateTime<Utc>) -> AppResult<Card>;
    /// Returns `false` if the card does not exist.
    async fn update(&self, id: i64, input: &CardInput, now: DateTime<Utc>) -> AppResult<bool>;
    /// Returns `false` if the card does not exist.
    async fn delete(&self, id: i64) -> AppResult<bool>;
    async fn list_tags(&self) -> AppResult<Vec<TagCount>>;
    /// Every card's id and word, alphabetical.
    async fn words(&self) -> AppResult<Vec<DeckWord>>;

    /// Cards with `due < cutoff`, most overdue first.
    async fn due_before(&self, cutoff: DateTime<Utc>, limit: i64) -> AppResult<Vec<Card>>;
    async fn count_due_before(&self, cutoff: DateTime<Utc>) -> AppResult<i64>;
    /// Due cards split into new / review / again. `today_start` is local
    /// midnight: a card last reviewed after it is in its relearning step.
    async fn due_breakdown(
        &self,
        cutoff: DateTime<Utc>,
        today_start: DateTime<Utc>,
    ) -> AppResult<DueBreakdown>;

    /// Saves the card's new FSRS state and the review log in one transaction.
    async fn save_review(&self, card: &Card, log: &NewReviewLog) -> AppResult<()>;
}

#[async_trait]
pub trait WritingRepository: Send + Sync {
    async fn list(&self) -> AppResult<Vec<PieceSummary>>;
    async fn get(&self, id: i64) -> AppResult<Option<Piece>>;
    /// Creates the piece plus an empty version 1. Returns the new id.
    async fn create(
        &self,
        kind: WritingKind,
        prompt: &str,
        written_on: NaiveDate,
        now: DateTime<Utc>,
    ) -> AppResult<i64>;
    async fn update(&self, id: i64, update: &PieceUpdate, now: DateTime<Utc>) -> AppResult<bool>;
    async fn delete(&self, id: i64) -> AppResult<bool>;

    async fn get_version(&self, id: i64) -> AppResult<Option<Version>>;
    async fn count_versions(&self, piece_id: i64) -> AppResult<i64>;
    /// New version whose body is a copy of the latest one.
    /// Returns `None` if the piece does not exist.
    async fn add_version(&self, piece_id: i64, now: DateTime<Utc>) -> AppResult<Option<Version>>;
    async fn update_version(
        &self,
        id: i64,
        update: &VersionUpdate,
        word_count: i64,
        now: DateTime<Utc>,
    ) -> AppResult<bool>;
    async fn delete_version(&self, id: i64) -> AppResult<bool>;

    /// Adds or replaces the piece's chart image. `false` if the piece does not exist.
    async fn save_image(
        &self,
        piece_id: i64,
        meta: &ImageMeta,
        data: &[u8],
        now: DateTime<Utc>,
    ) -> AppResult<bool>;
    async fn get_image(&self, piece_id: i64) -> AppResult<Option<StoredImage>>;
    /// `false` if the piece had no image.
    async fn delete_image(&self, piece_id: i64) -> AppResult<bool>;
}

/// The spaced-repetition algorithm. Plain (not async): it is pure maths.
pub trait Scheduler: Send + Sync {
    /// What each button would do. `memory` is `None` for a new card.
    fn next_states(&self, memory: Option<MemoryState>, elapsed_days: u32) -> AppResult<NextStates>;
}

// ---- Phase 2 ------------------------------------------------------------

#[async_trait]
pub trait PracticeRepository: Send + Sync {
    /// Newest first.
    async fn list(&self, limit: i64) -> AppResult<Vec<PracticeSession>>;
    async fn create(&self, practice: &NewPractice, now: DateTime<Utc>) -> AppResult<i64>;
    async fn get(&self, id: i64) -> AppResult<Option<PracticeSession>>;
    async fn update(&self, id: i64, practice: &NewPractice) -> AppResult<bool>;
    async fn delete(&self, id: i64) -> AppResult<bool>;
}

/// Read-only counts across reviews, writing and practice, for the heatmap.
#[async_trait]
pub trait ActivityRepository: Send + Sync {
    /// One entry per local date on or after `from` that had any activity.
    /// `level` is left at 0; the service fills it in.
    async fn daily_activity(&self, from: NaiveDate, tz: Tz) -> AppResult<Vec<DayActivity>>;
}

#[async_trait]
pub trait MistakeRepository: Send + Sync {
    async fn for_piece(&self, piece_id: i64) -> AppResult<Vec<MistakeCount>>;
    /// Replaces all of a piece's mistakes. `false` if the piece does not exist.
    async fn replace_for_piece(&self, piece_id: i64, mistakes: &[MistakeCount]) -> AppResult<bool>;
    async fn tags(&self) -> AppResult<Vec<MistakeTagSummary>>;
    /// Words written per period (oldest first) and mistake counts per period and tag.
    async fn trend_data(
        &self,
        bucket: TrendBucket,
    ) -> AppResult<(Vec<PeriodWords>, Vec<PeriodCount>)>;
}

#[async_trait]
pub trait AiFeedbackRepository: Send + Sync {
    /// Stores one call, usable or not. Returns the new row's id.
    async fn save(&self, call: AiCall<'_>) -> AppResult<i64>;
    /// Successful reviews of any version of the piece, newest first.
    async fn for_piece(&self, piece_id: i64) -> AppResult<Vec<AiFeedbackRecord>>;
    /// Tokens used per model since `since`, failed calls included.
    async fn usage_since(&self, since: DateTime<Utc>) -> AppResult<Vec<(String, i64)>>;
}

/// The AI model that reviews writing.
#[async_trait]
pub trait WritingReviewer: Send + Sync {
    /// Errors: `RateLimited` when the gateway's daily quota is used up,
    /// `Unavailable` when the gateway can't be reached or rejects the key.
    async fn review(&self, model: &str, request: &ReviewRequest) -> AppResult<ReviewOutcome>;
}
