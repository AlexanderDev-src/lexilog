//! Which calendar day it is for the user. Timestamps are stored in UTC, but
//! "today", "due today", the heatmap and the AI quota reset all follow the
//! user's time zone (`APP_TZ`).

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Today's date where the user lives.
pub fn today(now: DateTime<Utc>, tz: Tz) -> NaiveDate {
    now.with_timezone(&tz).date_naive()
}

/// The moment a local date starts (00:00 there), as UTC.
pub fn local_midnight(date: NaiveDate, tz: Tz) -> DateTime<Utc> {
    let midnight = date.and_time(NaiveTime::MIN);
    // `earliest()` handles DST changes, where a local time can be ambiguous or
    // missing. If midnight doesn't exist that day, treating it as UTC is close enough.
    tz.from_local_datetime(&midnight)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&midnight))
}

/// Local midnight at the end of today, in UTC. "Due today" means `due < this`.
pub fn end_of_today(now: DateTime<Utc>, tz: Tz) -> DateTime<Utc> {
    local_midnight(today(now, tz) + Duration::days(1), tz)
}

/// Number of calendar days (in the user's time zone) between two moments.
/// A review at 23:50 and one at 00:10 the next morning are 1 day apart.
pub fn calendar_days_between(from: DateTime<Utc>, to: DateTime<Utc>, tz: Tz) -> u32 {
    let from_day = from.with_timezone(&tz).date_naive();
    let to_day = to.with_timezone(&tz).date_naive();
    (to_day - from_day).num_days().max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bangkok() -> Tz {
        "Asia/Bangkok".parse().unwrap() // UTC+7, no DST
    }

    #[test]
    fn calendar_days_use_local_midnight() {
        // 23:50 and 00:10 Bangkok time, 20 minutes apart.
        let late = Utc.with_ymd_and_hms(2026, 9, 26, 16, 50, 0).unwrap();
        let early = Utc.with_ymd_and_hms(2026, 9, 26, 17, 10, 0).unwrap();
        assert_eq!(calendar_days_between(late, early, bangkok()), 1);
        assert_eq!(calendar_days_between(late, late, bangkok()), 0);
    }

    #[test]
    fn end_of_today_is_local_midnight() {
        let now = Utc.with_ymd_and_hms(2026, 9, 26, 3, 0, 0).unwrap(); // 10:00 Bangkok
        let end = end_of_today(now, bangkok());
        assert_eq!(end, Utc.with_ymd_and_hms(2026, 9, 26, 17, 0, 0).unwrap());
    }

    #[test]
    fn today_follows_the_time_zone() {
        // 18:00 UTC on the 26th is already 01:00 on the 27th in Bangkok.
        let now = Utc.with_ymd_and_hms(2026, 9, 26, 18, 0, 0).unwrap();
        assert_eq!(
            today(now, bangkok()),
            NaiveDate::from_ymd_opt(2026, 9, 27).unwrap()
        );
    }
}
