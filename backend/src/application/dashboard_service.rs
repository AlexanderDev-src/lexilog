use std::sync::Arc;

use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::Serialize;

use super::ports::{ActivityRepository, CardRepository, WritingRepository};
use crate::domain::activity::{DayActivity, heatmap_start};
use crate::domain::calendar::{end_of_today, local_midnight, today};
use crate::domain::card::{Card, DueBreakdown};
use crate::domain::error::AppResult;
use crate::domain::writing::PieceSummary;

/// Everything the home page shows, in one response.
#[derive(Debug, Serialize)]
pub struct Dashboard {
    pub today: NaiveDate,
    /// First date on the heatmap (a Monday).
    pub from: NaiveDate,
    pub due_today: i64,
    pub due: DueBreakdown,
    /// The first card in today's queue, shown on the dashboard.
    pub next_card: Option<Card>,
    /// The piece edited most recently.
    pub latest_piece: Option<PieceSummary>,
    /// Only days with activity; missing dates are empty cells.
    pub days: Vec<DayActivity>,
    pub active_days_year: usize,
    pub active_days_month: usize,
}

#[derive(Clone)]
pub struct DashboardService {
    cards: Arc<dyn CardRepository>,
    writing: Arc<dyn WritingRepository>,
    activity: Arc<dyn ActivityRepository>,
    tz: Tz,
}

impl DashboardService {
    pub fn new(
        cards: Arc<dyn CardRepository>,
        writing: Arc<dyn WritingRepository>,
        activity: Arc<dyn ActivityRepository>,
        tz: Tz,
    ) -> Self {
        Self {
            cards,
            writing,
            activity,
            tz,
        }
    }

    pub async fn dashboard(&self) -> AppResult<Dashboard> {
        let now = Utc::now();
        let today = today(now, self.tz);
        let from = heatmap_start(today);

        let cutoff = end_of_today(now, self.tz);
        let due_today = self.cards.count_due_before(cutoff).await?;
        let due = self
            .cards
            .due_breakdown(cutoff, local_midnight(today, self.tz))
            .await?;
        let next_card = self.cards.due_before(cutoff, 1).await?.into_iter().next();
        let latest_piece = self
            .writing
            .list()
            .await?
            .into_iter()
            .max_by(|a, b| a.updated_at.cmp(&b.updated_at));

        let days: Vec<DayActivity> = self
            .activity
            .daily_activity(from, self.tz)
            .await?
            .into_iter()
            .map(DayActivity::with_level)
            .collect();

        let active_days_year = days.iter().filter(|d| d.level > 0).count();
        let active_days_month = days
            .iter()
            .filter(|d| {
                d.level > 0 && d.date.year() == today.year() && d.date.month() == today.month()
            })
            .count();

        Ok(Dashboard {
            today,
            from,
            due_today,
            due,
            next_card,
            latest_piece,
            days,
            active_days_year,
            active_days_month,
        })
    }
}
