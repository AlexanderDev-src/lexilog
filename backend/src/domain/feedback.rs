//! AI feedback on a writing version: what we ask for, what we accept back.
//!
//! The model is told to answer with one JSON object. Its reply is untrusted
//! text, so `AiFeedback::from_model_reply` parses it and checks every field
//! before anything is stored or shown.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::image::StoredImage;
use super::mistakes::normalize_tag;
use super::writing::WritingKind;

/// Estimated IELTS bands, 0 to 9 in steps of 0.5.
/// `task` = Task Achievement (Task 1) or Task Response (Task 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bands {
    pub task: f64,
    pub coherence: f64,
    pub lexical: f64,
    pub grammar: f64,
    pub overall: f64,
}

/// One problem in the essay. `hint` is a question that points at the fix
/// without giving it away.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    /// Exact words copied from the essay, used to find and select them.
    pub quote: String,
    /// Mistake category, e.g. "missing plural -s".
    pub tag: String,
    pub hint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiFeedback {
    pub bands: Bands,
    pub summary: String,
    #[serde(default)]
    pub issues: Vec<Issue>,
    /// Open questions about content and structure.
    #[serde(default)]
    pub questions: Vec<String>,
}

pub const MAX_ISSUES: usize = 15;
pub const MAX_QUESTIONS: usize = 5;

impl AiFeedback {
    /// Parses and checks the model's reply. Models sometimes wrap the JSON in
    /// ``` fences or add a sentence around it, so this reads from the first
    /// `{` to the last `}`. The error text is sent back to the model when we
    /// ask it to try again.
    pub fn from_model_reply(reply: &str) -> Result<Self, String> {
        let start = reply.find('{').ok_or("the reply contains no JSON object")?;
        let end = reply
            .rfind('}')
            .ok_or("the reply contains no JSON object")?;
        if end < start {
            return Err("the reply contains no JSON object".into());
        }
        let parsed: AiFeedback = serde_json::from_str(&reply[start..=end])
            .map_err(|err| format!("the JSON does not match the required shape: {err}"))?;
        parsed.checked()
    }

    fn checked(self) -> Result<Self, String> {
        let bands = Bands {
            task: band("task", self.bands.task)?,
            coherence: band("coherence", self.bands.coherence)?,
            lexical: band("lexical", self.bands.lexical)?,
            grammar: band("grammar", self.bands.grammar)?,
            overall: band("overall", self.bands.overall)?,
        };

        let summary = self.summary.trim().to_string();
        if summary.is_empty() {
            return Err("summary is empty".into());
        }

        let issues = self
            .issues
            .into_iter()
            .map(|issue| Issue {
                quote: issue.quote.trim().to_string(),
                tag: normalize_tag(&issue.tag),
                hint: issue.hint.trim().to_string(),
            })
            .filter(|issue| !issue.quote.is_empty() && !issue.hint.is_empty())
            .take(MAX_ISSUES)
            .collect();

        let questions = self
            .questions
            .into_iter()
            .map(|q| q.trim().to_string())
            .filter(|q| !q.is_empty())
            .take(MAX_QUESTIONS)
            .collect();

        Ok(AiFeedback {
            bands,
            summary,
            issues,
            questions,
        })
    }
}

/// Rounds to the nearest half band and rejects anything outside 0–9.
fn band(name: &str, value: f64) -> Result<f64, String> {
    if !(0.0..=9.0).contains(&value) {
        return Err(format!(
            "band '{name}' must be between 0 and 9, got {value}"
        ));
    }
    Ok((value * 2.0).round() / 2.0)
}

/// Everything the reviewer needs about the piece being checked.
#[derive(Debug, Clone)]
pub struct ReviewRequest {
    pub kind: WritingKind,
    pub prompt: String,
    pub essay: String,
    pub version_no: i64,
    pub word_count: i64,
    /// The learner's existing mistake tags, so the model reuses them.
    pub known_tags: Vec<String>,
    /// The piece's chart, only when the model can read images.
    pub image: Option<StoredImage>,
}

/// What one review cost, and its result. `feedback` is an `Err` with the
/// reason when the model's reply could not be used even after a retry; the
/// tokens were still spent and still count towards the quota.
#[derive(Debug, Clone)]
pub struct ReviewOutcome {
    pub feedback: Result<AiFeedback, String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// One call to the reviewer, as stored in `ai_feedback`.
///
/// The `'a` lifetime says this struct only borrows `model` and `feedback`
/// from the caller for as long as it lives; nothing is copied.
#[derive(Debug, Clone, Copy)]
pub struct AiCall<'a> {
    pub version_id: i64,
    pub model: &'a str,
    /// `None` when the reply could not be used.
    pub feedback: Option<&'a AiFeedback>,
    pub with_image: bool,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: DateTime<Utc>,
}

/// A stored, successful review.
#[derive(Debug, Clone, Serialize)]
pub struct AiFeedbackRecord {
    pub id: i64,
    pub version_id: i64,
    pub model: String,
    pub feedback: AiFeedback,
    /// Whether the model saw the chart image.
    pub with_image: bool,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: DateTime<Utc>,
}

/// One model the app may use, with today's usage by this app.
#[derive(Debug, Clone, Serialize)]
pub struct ModelQuota {
    pub id: String,
    pub daily_limit: i64,
    pub used_today: i64,
    /// Can read images (marked `:vision` in AI_MODELS).
    pub vision: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiStatus {
    pub enabled: bool,
    pub default_model: Option<String>,
    pub models: Vec<ModelQuota>,
    /// Next local midnight, when the gateway's quotas reset.
    pub resets_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"bands":{"task":6.5,"coherence":6.0,"lexical":6.4,"grammar":6.0,"overall":6.5},
        "summary":" Clear position. ",
        "issues":[{"quote":"work in team","tag":"Missing plural -s","hint":"One team, or many?"},
                  {"quote":"","tag":"x","hint":"dropped: no quote"}],
        "questions":["What example supports paragraph 3?"]}"#;

    #[test]
    fn parses_and_cleans_a_good_reply() {
        let feedback = AiFeedback::from_model_reply(GOOD).unwrap();
        assert_eq!(feedback.bands.lexical, 6.5); // rounded to a half band
        assert_eq!(feedback.summary, "Clear position.");
        assert_eq!(feedback.issues.len(), 1);
        assert_eq!(feedback.issues[0].tag, "missing plural -s");
    }

    #[test]
    fn accepts_json_inside_code_fences() {
        let wrapped = format!("Here is the feedback:\n```json\n{GOOD}\n```");
        assert!(AiFeedback::from_model_reply(&wrapped).is_ok());
    }

    #[test]
    fn rejects_bad_replies() {
        assert!(AiFeedback::from_model_reply("Sorry, I can't help.").is_err());
        assert!(AiFeedback::from_model_reply(r#"{"summary":"no bands"}"#).is_err());
        let out_of_range = GOOD.replace("\"task\":6.5", "\"task\":12");
        assert!(AiFeedback::from_model_reply(&out_of_range).is_err());
    }
}
