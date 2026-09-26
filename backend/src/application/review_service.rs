use std::sync::Arc;

use chrono::{DateTime, SubsecRound, Utc};
use chrono_tz::Tz;
use serde::Serialize;

use super::ports::{CardRepository, Scheduler};
use crate::domain::calendar::{calendar_days_between, end_of_today};
use crate::domain::card::Card;
use crate::domain::error::{AppError, AppResult};
use crate::domain::review::{Intervals, NewReviewLog, Rating};

/// A due card plus how long each button would postpone it.
#[derive(Debug, Serialize)]
pub struct DueCard {
    /// `flatten` puts the card's fields at the top level of the JSON object
    /// instead of nesting them under "card".
    #[serde(flatten)]
    pub card: Card,
    pub preview: IntervalPreview,
}

/// Seconds until the next review for each button, e.g. `good: 259200` = 3 days.
#[derive(Debug, Serialize)]
pub struct IntervalPreview {
    pub again: i64,
    pub hard: i64,
    pub good: i64,
    pub easy: i64,
}

impl From<Intervals> for IntervalPreview {
    fn from(intervals: Intervals) -> Self {
        Self {
            again: intervals.again.num_seconds(),
            hard: intervals.hard.num_seconds(),
            good: intervals.good.num_seconds(),
            easy: intervals.easy.num_seconds(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DueQueue {
    /// All cards due today, even if `cards` holds fewer because of the limit.
    pub total: i64,
    pub cards: Vec<DueCard>,
}

/// Review sessions: which cards are due, and what happens when you rate one.
#[derive(Clone)]
pub struct ReviewService {
    cards: Arc<dyn CardRepository>,
    scheduler: Arc<dyn Scheduler>,
    tz: Tz,
}

impl ReviewService {
    pub fn new(cards: Arc<dyn CardRepository>, scheduler: Arc<dyn Scheduler>, tz: Tz) -> Self {
        Self {
            cards,
            scheduler,
            tz,
        }
    }

    /// Cards due before local midnight tonight, most overdue first.
    pub async fn due(&self, limit: i64) -> AppResult<DueQueue> {
        let now = Utc::now();
        let cutoff = end_of_today(now, self.tz);
        let total = self.cards.count_due_before(cutoff).await?;

        let mut due_cards = Vec::new();
        for card in self.cards.due_before(cutoff, limit.clamp(1, 500)).await? {
            let preview = self.intervals(&card, now)?.into();
            due_cards.push(DueCard { card, preview });
        }
        Ok(DueQueue {
            total,
            cards: due_cards,
        })
    }

    /// Applies a rating: asks FSRS for the new memory state, picks the next
    /// due time, and stores the card together with a review log entry.
    pub async fn review(
        &self,
        card_id: i64,
        rating: Rating,
        duration_ms: Option<i64>,
    ) -> AppResult<Card> {
        let mut card = self
            .cards
            .get(card_id)
            .await?
            .ok_or(AppError::NotFound("card"))?;
        // Whole seconds, matching what the database stores.
        let now = Utc::now().trunc_subsecs(0);

        let elapsed_days = self.elapsed_days(&card, now);
        let before = card.memory();
        let next = self.scheduler.next_states(before, elapsed_days)?;
        let after = next.for_rating(rating).memory;
        let wait = Intervals::from_next_states(&next).for_rating(rating);

        // A lapse = forgetting a card you had already learned on an earlier day.
        // Pressing Again twice in one session is not counted twice.
        if rating == Rating::Again && before.is_some() && elapsed_days > 0 {
            card.lapses += 1;
        }
        card.stability = Some(after.stability);
        card.difficulty = Some(after.difficulty);
        card.due = now + wait;
        card.last_review = Some(now);
        card.reps += 1;

        let log = NewReviewLog {
            card_id,
            rating,
            reviewed_at: now,
            elapsed_days,
            before,
            after,
            scheduled_days: wait.num_seconds() as f64 / 86_400.0,
            // Ignore nonsense values like negative times.
            duration_ms: duration_ms.filter(|ms| *ms >= 0),
        };
        self.cards.save_review(&card, &log).await?;
        Ok(card)
    }

    fn intervals(&self, card: &Card, now: DateTime<Utc>) -> AppResult<Intervals> {
        let next = self
            .scheduler
            .next_states(card.memory(), self.elapsed_days(card, now))?;
        Ok(Intervals::from_next_states(&next))
    }

    fn elapsed_days(&self, card: &Card, now: DateTime<Utc>) -> u32 {
        card.last_review
            .map(|last| calendar_days_between(last, now, self.tz))
            .unwrap_or(0)
    }
}
