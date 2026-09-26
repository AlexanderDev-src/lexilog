//! The practice calendar (heatmap) on the dashboard.
//!
//! Deliberately no streaks: every active day counts on its own, and a day
//! off never resets anything.

use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;

/// Everything practised on one local date.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DayActivity {
    pub date: NaiveDate,
    pub reviews: i64,
    /// Writing versions written or edited that day.
    pub writing: i64,
    pub practice_minutes: i64,
    /// Heatmap shade, 0 (nothing) to 4 (a lot).
    pub level: u8,
}

impl DayActivity {
    pub fn empty(date: NaiveDate) -> Self {
        Self {
            date,
            reviews: 0,
            writing: 0,
            practice_minutes: 0,
            level: 0,
        }
    }

    /// One number for "how much practice": a review is about a minute, a
    /// logged minute is a minute, and a writing session counts as 20.
    pub fn score(&self) -> i64 {
        self.reviews + self.practice_minutes + 20 * self.writing
    }

    /// Recomputes `level` from the counts.
    pub fn with_level(mut self) -> Self {
        self.level = level(self.score());
        self
    }
}

/// Maps a day's score to a heatmap shade.
pub fn level(score: i64) -> u8 {
    match score {
        s if s <= 0 => 0,
        1..=15 => 1,
        16..=40 => 2,
        41..=80 => 3,
        _ => 4,
    }
}

/// First day shown on the heatmap: the Monday 52 weeks before this week's
/// Monday, so the grid is exactly 53 full week columns ending with this week.
pub fn heatmap_start(today: NaiveDate) -> NaiveDate {
    let this_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    this_monday - Duration::weeks(52)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn level_thresholds() {
        assert_eq!(level(0), 0);
        assert_eq!(level(1), 1);
        assert_eq!(level(15), 1);
        assert_eq!(level(16), 2);
        assert_eq!(level(80), 3);
        assert_eq!(level(81), 4);
    }

    #[test]
    fn one_writing_session_is_a_solid_day() {
        let day = DayActivity {
            writing: 1,
            ..DayActivity::empty(date(2026, 9, 26))
        }
        .with_level();
        assert_eq!(day.level, 2);
    }

    #[test]
    fn heatmap_starts_on_a_monday_53_weeks_back() {
        // Saturday 26 Sep 2026 -> this week's Monday is 21 Sep 2026.
        let start = heatmap_start(date(2026, 9, 26));
        assert_eq!(start, date(2025, 9, 22));
        assert_eq!(start.weekday(), chrono::Weekday::Mon);
        // A Monday itself starts its own column.
        assert_eq!(heatmap_start(date(2026, 9, 21)), date(2025, 9, 22));
    }
}
