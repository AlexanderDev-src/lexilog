use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{NaiveDate, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use sqlx::SqlitePool;

use super::database::timestamp;
use crate::application::ports::ActivityRepository;
use crate::domain::activity::DayActivity;
use crate::domain::calendar::local_midnight;
use crate::domain::error::AppResult;

pub struct SqliteActivityRepository {
    pool: SqlitePool,
}

impl SqliteActivityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// SQLite's `date(ts, '+25200 seconds')` shifts a UTC timestamp into local
/// time before taking the date. The offset is today's; in zones with
/// daylight saving, activity within an hour of midnight may land on the
/// neighbouring day. Asia/Bangkok has no DST, so it is exact there.
fn sqlite_offset(tz: Tz) -> String {
    let offset = tz.offset_from_utc_datetime(&Utc::now().naive_utc()).fix();
    format!("{:+} seconds", offset.local_minus_utc())
}

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    async fn daily_activity(&self, from: NaiveDate, tz: Tz) -> AppResult<Vec<DayActivity>> {
        let offset = sqlite_offset(tz);
        let since = timestamp(local_midnight(from, tz));

        let reviews = sqlx::query_as::<_, (NaiveDate, i64)>(
            "SELECT date(reviewed_at, ?1) AS day, COUNT(*)
               FROM review_logs
              WHERE reviewed_at >= ?2
              GROUP BY day",
        )
        .bind(&offset)
        .bind(&since)
        .fetch_all(&self.pool)
        .await?;

        // A version counts on the day it was created and on the day it was
        // last edited (UNION removes the double when both are the same day).
        // Empty versions don't count.
        let writing = sqlx::query_as::<_, (NaiveDate, i64)>(
            "SELECT day, COUNT(*) FROM (
                 SELECT id, date(created_at, ?1) AS day FROM writing_versions
                  WHERE created_at >= ?2 AND word_count > 0
                 UNION
                 SELECT id, date(updated_at, ?1) AS day FROM writing_versions
                  WHERE updated_at >= ?2 AND word_count > 0
             )
             GROUP BY day",
        )
        .bind(&offset)
        .bind(&since)
        .fetch_all(&self.pool)
        .await?;

        let practice = sqlx::query_as::<_, (NaiveDate, i64)>(
            "SELECT practiced_on, SUM(minutes)
               FROM practice_sessions
              WHERE practiced_on >= ?
              GROUP BY practiced_on",
        )
        .bind(from)
        .fetch_all(&self.pool)
        .await?;

        // Merge the three lists by date. BTreeMap keeps the dates sorted.
        let mut days: BTreeMap<NaiveDate, DayActivity> = BTreeMap::new();
        for (date, n) in reviews {
            days.entry(date)
                .or_insert_with(|| DayActivity::empty(date))
                .reviews = n;
        }
        for (date, n) in writing {
            days.entry(date)
                .or_insert_with(|| DayActivity::empty(date))
                .writing = n;
        }
        for (date, minutes) in practice {
            days.entry(date)
                .or_insert_with(|| DayActivity::empty(date))
                .practice_minutes = minutes;
        }
        Ok(days.into_values().collect())
    }
}
