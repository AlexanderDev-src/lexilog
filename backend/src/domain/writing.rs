use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::{AppError, AppResult};

/// The type of writing piece. In JSON: "task1", "task2" or "paragraph".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WritingKind {
    Task1,
    Task2,
    Paragraph,
}

impl WritingKind {
    /// The text stored in the `kind` column.
    pub fn as_str(self) -> &'static str {
        match self {
            WritingKind::Task1 => "task1",
            WritingKind::Task2 => "task2",
            WritingKind::Paragraph => "paragraph",
        }
    }

    pub fn parse(text: &str) -> AppResult<Self> {
        match text {
            "task1" => Ok(WritingKind::Task1),
            "task2" => Ok(WritingKind::Task2),
            "paragraph" => Ok(WritingKind::Paragraph),
            other => Err(AppError::Internal(format!(
                "unknown writing kind '{other}'"
            ))),
        }
    }
}

/// A writing piece with all of its versions (oldest first).
#[derive(Debug, Clone, Serialize)]
pub struct Piece {
    pub id: i64,
    pub kind: WritingKind,
    pub prompt: String,
    pub written_on: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub versions: Vec<Version>,
}

/// One row in the writing list.
#[derive(Debug, Clone, Serialize)]
pub struct PieceSummary {
    pub id: i64,
    pub kind: WritingKind,
    pub prompt: String,
    pub written_on: NaiveDate,
    pub version_count: i64,
    pub latest_word_count: i64,
    /// Timer seconds on the latest version, if the timer was used.
    pub latest_seconds_spent: Option<i64>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Version {
    pub id: i64,
    pub piece_id: i64,
    pub version_no: i64,
    pub body: String,
    pub word_count: i64,
    pub seconds_spent: Option<i64>,
    pub feedback: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Body of "create piece". `written_on` defaults to today.
#[derive(Debug, Clone, Deserialize)]
pub struct NewPiece {
    pub kind: WritingKind,
    #[serde(default)]
    pub prompt: String,
    pub written_on: Option<NaiveDate>,
}

/// Body of "edit piece".
#[derive(Debug, Clone, Deserialize)]
pub struct PieceUpdate {
    pub kind: WritingKind,
    #[serde(default)]
    pub prompt: String,
    pub written_on: NaiveDate,
}

/// Body of "save version" (autosave). Fields left out stay unchanged.
#[derive(Debug, Clone, Deserialize)]
pub struct VersionUpdate {
    pub body: String,
    pub seconds_spent: Option<i64>,
    pub feedback: Option<String>,
}

/// Word count the IELTS way: anything separated by whitespace is one word,
/// so "well-known" is 1 word and "25%" is 1 word.
/// The frontend uses the same rule for its live counter.
pub fn word_count(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_count_splits_on_any_whitespace() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("   \n "), 0);
        assert_eq!(word_count("A well-known  problem,\nin 2026."), 5);
    }

    #[test]
    fn kind_round_trips_through_text() {
        for kind in [
            WritingKind::Task1,
            WritingKind::Task2,
            WritingKind::Paragraph,
        ] {
            assert_eq!(WritingKind::parse(kind.as_str()).unwrap(), kind);
        }
    }
}
