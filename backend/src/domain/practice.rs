//! Practice done outside the app: listening, reading, speaking...

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::{AppError, AppResult};

/// In JSON: "listening", "reading", "speaking" or "other".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Skill {
    Listening,
    Reading,
    Speaking,
    Other,
}

impl Skill {
    pub fn as_str(self) -> &'static str {
        match self {
            Skill::Listening => "listening",
            Skill::Reading => "reading",
            Skill::Speaking => "speaking",
            Skill::Other => "other",
        }
    }

    pub fn parse(text: &str) -> AppResult<Self> {
        match text {
            "listening" => Ok(Skill::Listening),
            "reading" => Ok(Skill::Reading),
            "speaking" => Ok(Skill::Speaking),
            "other" => Ok(Skill::Other),
            other => Err(AppError::Internal(format!("unknown skill '{other}'"))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PracticeSession {
    pub id: i64,
    pub practiced_on: NaiveDate,
    pub skill: Skill,
    pub minutes: i64,
    pub note: String,
    pub created_at: DateTime<Utc>,
}

/// Body of "log practice" and "edit practice". The date defaults to today.
#[derive(Debug, Clone, Deserialize)]
pub struct PracticeInput {
    pub practiced_on: Option<NaiveDate>,
    pub skill: Skill,
    pub minutes: i64,
    #[serde(default)]
    pub note: String,
}

/// A checked `PracticeInput`, ready to store.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPractice {
    pub practiced_on: NaiveDate,
    pub skill: Skill,
    pub minutes: i64,
    pub note: String,
}

/// 10 hours: anything longer is almost certainly a typo.
pub const MAX_MINUTES: i64 = 600;

impl PracticeInput {
    pub fn validate(self, today: NaiveDate) -> AppResult<NewPractice> {
        if !(1..=MAX_MINUTES).contains(&self.minutes) {
            return Err(AppError::Validation(format!(
                "minutes must be between 1 and {MAX_MINUTES}"
            )));
        }
        let practiced_on = self.practiced_on.unwrap_or(today);
        if practiced_on > today {
            return Err(AppError::Validation(
                "the date cannot be in the future".into(),
            ));
        }
        Ok(NewPractice {
            practiced_on,
            skill: self.skill,
            minutes: self.minutes,
            note: self.note.trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 26).unwrap()
    }

    fn input(minutes: i64, practiced_on: Option<NaiveDate>) -> PracticeInput {
        PracticeInput {
            practiced_on,
            skill: Skill::Listening,
            minutes,
            note: "  Cambridge 18 test 3  ".into(),
        }
    }

    #[test]
    fn date_defaults_to_today_and_note_is_trimmed() {
        let practice = input(30, None).validate(today()).unwrap();
        assert_eq!(practice.practiced_on, today());
        assert_eq!(practice.note, "Cambridge 18 test 3");
    }

    #[test]
    fn rejects_bad_minutes_and_future_dates() {
        assert!(input(0, None).validate(today()).is_err());
        assert!(input(MAX_MINUTES + 1, None).validate(today()).is_err());
        let tomorrow = today().succ_opt();
        assert!(input(30, tomorrow).validate(today()).is_err());
    }
}
