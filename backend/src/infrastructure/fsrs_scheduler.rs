use fsrs::FSRS;

use crate::application::ports::Scheduler;
use crate::domain::error::{AppError, AppResult};
use crate::domain::review::{MemoryState, NextState, NextStates};

/// `Scheduler` backed by the official `fsrs` crate (FSRS-6).
///
/// Uses the default parameters for now. Once there are enough review logs,
/// the crate's optimizer can compute personal parameters to pass in here.
pub struct FsrsScheduler {
    fsrs: FSRS,
    desired_retention: f32,
}

impl FsrsScheduler {
    /// `desired_retention` = target chance of remembering a card when it
    /// comes due. 0.9 is the usual default: higher means more reviews.
    pub fn new(desired_retention: f32) -> Self {
        Self {
            fsrs: FSRS::default(),
            desired_retention,
        }
    }
}

impl Scheduler for FsrsScheduler {
    fn next_states(&self, memory: Option<MemoryState>, elapsed_days: u32) -> AppResult<NextStates> {
        // Our domain uses f64; the crate uses f32. `as` converts between them.
        let memory = memory.map(|m| fsrs::MemoryState {
            stability: m.stability as f32,
            difficulty: m.difficulty as f32,
        });

        let next = self
            .fsrs
            .next_states(memory, self.desired_retention, elapsed_days)
            .map_err(|err| AppError::Internal(format!("fsrs: {err:?}")))?;

        let convert = |state: fsrs::ItemState| NextState {
            memory: MemoryState {
                stability: state.memory.stability as f64,
                difficulty: state.memory.difficulty as f64,
            },
            interval_days: state.interval as f64,
        };

        Ok(NextStates {
            again: convert(next.again),
            hard: convert(next.hard),
            good: convert(next.good),
            easy: convert(next.easy),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_card_gets_increasing_intervals() {
        let next = FsrsScheduler::new(0.9).next_states(None, 0).unwrap();
        assert!(next.again.interval_days < next.hard.interval_days);
        assert!(next.hard.interval_days < next.good.interval_days);
        assert!(next.good.interval_days < next.easy.interval_days);
    }

    #[test]
    fn good_review_grows_stability() {
        let scheduler = FsrsScheduler::new(0.9);
        let first = scheduler.next_states(None, 0).unwrap().good.memory;
        let second = scheduler.next_states(Some(first), 3).unwrap().good.memory;
        assert!(second.stability > first.stability);
    }
}
