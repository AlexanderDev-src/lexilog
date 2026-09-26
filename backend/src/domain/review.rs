//! Spaced-repetition rules that sit on top of FSRS.
//!
//! FSRS itself (the maths) lives behind the `Scheduler` port. This file holds
//! the decisions *we* make with its output: the 10-minute relearn step and
//! rounding to whole days. Day boundaries live in `calendar.rs`.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// The four review buttons.
///
/// `#[serde(try_from = "u8", into = "u8")]` makes JSON use plain numbers
/// 1..=4 and rejects anything else with a clear error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum Rating {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl TryFrom<u8> for Rating {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Rating::Again),
            2 => Ok(Rating::Hard),
            3 => Ok(Rating::Good),
            4 => Ok(Rating::Easy),
            other => Err(format!("rating must be 1-4, got {other}")),
        }
    }
}

impl From<Rating> for u8 {
    fn from(rating: Rating) -> u8 {
        rating as u8
    }
}

/// How well a card is remembered, in FSRS terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryState {
    /// Days until recall probability drops to 90%.
    pub stability: f64,
    /// 1 (easy) to 10 (hard).
    pub difficulty: f64,
}

/// FSRS's prediction for one button.
#[derive(Debug, Clone, Copy)]
pub struct NextState {
    pub memory: MemoryState,
    pub interval_days: f64,
}

/// FSRS's prediction for all four buttons.
#[derive(Debug, Clone, Copy)]
pub struct NextStates {
    pub again: NextState,
    pub hard: NextState,
    pub good: NextState,
    pub easy: NextState,
}

impl NextStates {
    pub fn for_rating(&self, rating: Rating) -> NextState {
        match rating {
            Rating::Again => self.again,
            Rating::Hard => self.hard,
            Rating::Good => self.good,
            Rating::Easy => self.easy,
        }
    }
}

/// After "Again" the card comes back in the same session.
pub const RELEARN_STEP_MINUTES: i64 = 10;

/// Real waiting time before the next review, per button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intervals {
    pub again: Duration,
    pub hard: Duration,
    pub good: Duration,
    pub easy: Duration,
}

impl Intervals {
    /// Turns FSRS's fractional-day intervals into what we actually use:
    /// Again = 10 minutes, others = whole days (at least 1), and each button
    /// strictly longer than the one before so the buttons never show the same value.
    pub fn from_next_states(next: &NextStates) -> Self {
        let hard = whole_days(next.hard.interval_days);
        let good = whole_days(next.good.interval_days).max(hard + 1);
        let easy = whole_days(next.easy.interval_days).max(good + 1);
        Intervals {
            again: Duration::minutes(RELEARN_STEP_MINUTES),
            hard: Duration::days(hard),
            good: Duration::days(good),
            easy: Duration::days(easy),
        }
    }

    pub fn for_rating(&self, rating: Rating) -> Duration {
        match rating {
            Rating::Again => self.again,
            Rating::Hard => self.hard,
            Rating::Good => self.good,
            Rating::Easy => self.easy,
        }
    }
}

fn whole_days(days: f64) -> i64 {
    (days.round() as i64).max(1)
}

/// A finished review, ready to be stored in `review_logs`.
#[derive(Debug, Clone)]
pub struct NewReviewLog {
    pub card_id: i64,
    pub rating: Rating,
    pub reviewed_at: DateTime<Utc>,
    pub elapsed_days: u32,
    pub before: Option<MemoryState>,
    pub after: MemoryState,
    pub scheduled_days: f64,
    pub duration_ms: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(interval_days: f64) -> NextState {
        NextState {
            memory: MemoryState {
                stability: 1.0,
                difficulty: 5.0,
            },
            interval_days,
        }
    }

    #[test]
    fn intervals_are_whole_days_and_increasing() {
        let next = NextStates {
            again: state(0.2),
            hard: state(0.4),
            good: state(1.3),
            easy: state(1.4),
        };
        let intervals = Intervals::from_next_states(&next);
        assert_eq!(intervals.again, Duration::minutes(10));
        assert_eq!(intervals.hard, Duration::days(1));
        assert_eq!(intervals.good, Duration::days(2));
        assert_eq!(intervals.easy, Duration::days(3));
    }

    #[test]
    fn rating_rejects_out_of_range() {
        assert!(Rating::try_from(0).is_err());
        assert!(Rating::try_from(5).is_err());
        assert_eq!(Rating::try_from(3), Ok(Rating::Good));
    }
}
