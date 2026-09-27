use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::{AppError, AppResult};
use super::review::MemoryState;

/// A vocabulary card, including its FSRS scheduling state.
#[derive(Debug, Clone, Serialize)]
pub struct Card {
    pub id: i64,
    pub word: String,
    /// One of [`PARTS_OF_SPEECH`], or "" when not set.
    pub part_of_speech: String,
    pub meaning: String,
    pub example: String,
    pub source: String,
    pub tags: Vec<String>,
    /// `None` until the first review.
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub due: DateTime<Utc>,
    pub last_review: Option<DateTime<Utc>>,
    pub reps: i64,
    pub lapses: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Card {
    /// The FSRS memory state, or `None` for a new card.
    pub fn memory(&self) -> Option<MemoryState> {
        // `zip` turns (Some(a), Some(b)) into Some((a, b)); anything else is None.
        self.stability
            .zip(self.difficulty)
            .map(|(stability, difficulty)| MemoryState {
                stability,
                difficulty,
            })
    }
}

/// The part-of-speech codes a card may have: noun, verb, adjective, adverb,
/// preposition, conjunction, phrasal verb, phrase, idiom.
pub const PARTS_OF_SPEECH: [&str; 9] = [
    "n", "v", "adj", "adv", "prep", "conj", "phrv", "phrase", "idiom",
];

/// Body of "create card" and "edit card". Only `word` is required;
/// `#[serde(default)]` fills missing fields with "" or an empty list.
#[derive(Debug, Clone, Deserialize)]
pub struct CardInput {
    pub word: String,
    #[serde(default)]
    pub part_of_speech: String,
    #[serde(default)]
    pub meaning: String,
    #[serde(default)]
    pub example: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl CardInput {
    /// Trims every field, lowercases and de-duplicates tags,
    /// and rejects an empty word or an unknown part of speech.
    pub fn validate(self) -> AppResult<Self> {
        let word = self.word.trim().to_string();
        if word.is_empty() {
            return Err(AppError::Validation("word is required".into()));
        }

        let part_of_speech = self.part_of_speech.trim().to_string();
        if !part_of_speech.is_empty() && !PARTS_OF_SPEECH.contains(&part_of_speech.as_str()) {
            return Err(AppError::Validation(format!(
                "unknown part of speech: {part_of_speech}"
            )));
        }

        let mut tags: Vec<String> = Vec::new();
        for tag in self.tags {
            let tag = tag.trim().to_lowercase();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }

        Ok(Self {
            word,
            part_of_speech,
            meaning: self.meaning.trim().to_string(),
            example: self.example.trim().to_string(),
            source: self.source.trim().to_string(),
            tags,
        })
    }
}

/// Search options for the card list, read from the query string.
#[derive(Debug, Clone, Deserialize)]
pub struct CardFilter {
    /// Matches word, meaning or example (case-insensitive).
    pub q: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl CardFilter {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).clamp(1, 500)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

/// Cards due today, split by kind (for the dashboard).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DueBreakdown {
    /// Never reviewed.
    pub new: i64,
    /// Scheduled on an earlier day.
    pub review: i64,
    /// Pressed Again today; coming back after the 10-minute step.
    pub again: i64,
}

/// Just the word of every card, so the writing editor can underline the
/// deck words used in an essay.
#[derive(Debug, Clone, Serialize)]
pub struct DeckWord {
    pub id: i64,
    pub word: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagCount {
    pub name: String,
    pub card_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(word: &str, tags: &[&str]) -> CardInput {
        CardInput {
            word: word.into(),
            part_of_speech: String::new(),
            meaning: " to reduce ".into(),
            example: String::new(),
            source: String::new(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    #[test]
    fn empty_word_is_rejected() {
        assert!(input("   ", &[]).validate().is_err());
    }

    #[test]
    fn fields_are_trimmed_and_tags_normalised() {
        let card = input(" mitigate ", &["Environment", "environment ", "", "policy"])
            .validate()
            .unwrap();
        assert_eq!(card.word, "mitigate");
        assert_eq!(card.meaning, "to reduce");
        assert_eq!(card.tags, vec!["environment", "policy"]);
    }

    #[test]
    fn part_of_speech_must_be_known() {
        let mut card = input("mitigate", &[]);
        card.part_of_speech = " v ".into();
        assert_eq!(card.clone().validate().unwrap().part_of_speech, "v");

        card.part_of_speech = "verb".into();
        assert!(card.validate().is_err());
    }
}
