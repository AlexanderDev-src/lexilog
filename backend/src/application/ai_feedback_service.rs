use std::sync::Arc;

use chrono::{SubsecRound, Utc};
use chrono_tz::Tz;

use super::ports::{AiFeedbackRepository, MistakeRepository, WritingRepository, WritingReviewer};
use crate::domain::calendar::{end_of_today, local_midnight, today};
use crate::domain::error::{AppError, AppResult};
use crate::domain::feedback::{
    AiCall, AiFeedbackRecord, AiStatus, CallTrace, ModelQuota, ReviewRequest,
};

/// A model the app may use, its daily token limit on the gateway, and
/// whether it can read images.
#[derive(Debug, Clone)]
pub struct ModelLimit {
    pub id: String,
    pub daily_limit: i64,
    pub vision: bool,
}

/// Below this many words there is nothing useful to review, so don't spend quota.
const MIN_WORDS: i64 = 30;

/// AI feedback on writing versions.
#[derive(Clone)]
pub struct AiFeedbackService {
    /// `None` when no API key is configured: the feature is switched off.
    reviewer: Option<Arc<dyn WritingReviewer>>,
    feedback: Arc<dyn AiFeedbackRepository>,
    writing: Arc<dyn WritingRepository>,
    mistakes: Arc<dyn MistakeRepository>,
    models: Vec<ModelLimit>,
    default_model: Option<String>,
    tz: Tz,
}

impl AiFeedbackService {
    pub fn new(
        reviewer: Option<Arc<dyn WritingReviewer>>,
        feedback: Arc<dyn AiFeedbackRepository>,
        writing: Arc<dyn WritingRepository>,
        mistakes: Arc<dyn MistakeRepository>,
        models: Vec<ModelLimit>,
        default_model: Option<String>,
        tz: Tz,
    ) -> Self {
        Self {
            reviewer,
            feedback,
            writing,
            mistakes,
            models,
            default_model,
            tz,
        }
    }

    /// Whether AI feedback is on, and each model's usage by this app today.
    pub async fn status(&self) -> AppResult<AiStatus> {
        let now = Utc::now();
        let usage = self.usage_today().await?;
        let models = self
            .models
            .iter()
            .map(|m| ModelQuota {
                id: m.id.clone(),
                daily_limit: m.daily_limit,
                used_today: used_by(&usage, &m.id),
                vision: m.vision,
            })
            .collect();
        Ok(AiStatus {
            enabled: self.reviewer.is_some(),
            default_model: self.default_model.clone(),
            models,
            resets_at: end_of_today(now, self.tz),
        })
    }

    pub async fn for_piece(&self, piece_id: i64) -> AppResult<Vec<AiFeedbackRecord>> {
        self.feedback.for_piece(piece_id).await
    }

    /// Sends one version to the model and stores the result.
    pub async fn review_version(
        &self,
        version_id: i64,
        model: Option<String>,
    ) -> AppResult<AiFeedbackRecord> {
        let reviewer = self.reviewer.as_ref().ok_or_else(|| {
            AppError::Unavailable(
                "AI feedback is not set up: add AI_API_KEY to the .env file".into(),
            )
        })?;

        // Pick the model: the one asked for, else the default. It must be configured.
        let model = model
            .or_else(|| self.default_model.clone())
            .ok_or_else(|| AppError::Unavailable("no AI model is configured".into()))?;
        let limit =
            self.models.iter().find(|m| m.id == model).ok_or_else(|| {
                AppError::Validation(format!("model '{model}' is not in AI_MODELS"))
            })?;

        let version = self
            .writing
            .get_version(version_id)
            .await?
            .ok_or(AppError::NotFound("version"))?;
        if version.word_count < MIN_WORDS {
            return Err(AppError::Validation(format!(
                "write at least {MIN_WORDS} words before asking for feedback"
            )));
        }
        let piece = self
            .writing
            .get(version.piece_id)
            .await?
            .ok_or(AppError::NotFound("piece"))?;

        // Our own count is a lower bound (the key is used elsewhere too), but
        // if even that is over the limit, the gateway would refuse anyway.
        let used = used_by(&self.usage_today().await?, &model);
        if used >= limit.daily_limit {
            return Err(AppError::RateLimited(format!(
                "today's {model} quota is used up; it resets at midnight"
            )));
        }

        // The chart goes along only if the model can read it. A text-only
        // model still gets the essay; the editor has already warned about it.
        let image = if limit.vision {
            self.writing.get_image(piece.id).await?
        } else {
            None
        };
        let with_image = image.is_some();

        let known_tags = self
            .mistakes
            .tags()
            .await?
            .into_iter()
            .map(|t| t.name)
            .collect();
        let request = ReviewRequest {
            kind: piece.kind,
            prompt: piece.prompt,
            essay: version.body,
            version_no: version.version_no,
            word_count: version.word_count,
            known_tags,
            image,
        };

        let outcome = reviewer.review(&model, &request).await;

        // Store every call, failed ones too: their tokens count towards the
        // quota, and the AI log shows what went wrong.
        let now = Utc::now().trunc_subsecs(0);
        let error = outcome.feedback.as_ref().err().map(ToString::to_string);
        let id = self
            .feedback
            .save(AiCall {
                version_id,
                model: &model,
                feedback: outcome.feedback.as_ref().ok(),
                error: error.as_deref(),
                with_image,
                trace: &outcome.trace,
                created_at: now,
            })
            .await?;
        let summary = call_summary(id, &model, version_id, with_image, &outcome.trace);
        match &error {
            None => log::info!("{summary}: ok"),
            Some(error) => log::warn!("{summary}: failed: {error}"),
        }

        // `?` hands a failed call's error (quota, gateway, unusable reply) to
        // the caller. Moving `feedback` out of `outcome` still leaves
        // `outcome.trace` usable below.
        let feedback = outcome.feedback?;
        Ok(AiFeedbackRecord {
            id,
            version_id,
            model,
            feedback,
            with_image,
            input_tokens: outcome.trace.input_tokens,
            output_tokens: outcome.trace.output_tokens,
            created_at: now,
        })
    }

    async fn usage_today(&self) -> AppResult<Vec<(String, i64)>> {
        let start = local_midnight(today(Utc::now(), self.tz), self.tz);
        self.feedback.usage_since(start).await
    }
}

/// The server-log line for one AI call (`docker compose logs app`), e.g.
/// "AI call #12: claude-sonnet-5, version 3, with chart, HTTP 200,
/// 1500 + 300 tokens, 12.4 s, 1 request". No essay text, no key.
fn call_summary(
    id: i64,
    model: &str,
    version_id: i64,
    with_image: bool,
    trace: &CallTrace,
) -> String {
    let chart = if with_image { ", with chart" } else { "" };
    let status = match trace.http_status {
        Some(code) => format!("HTTP {code}"),
        None => "no answer".into(),
    };
    let requests = if trace.attempts == 1 {
        "1 request".to_string()
    } else {
        format!("{} requests", trace.attempts)
    };
    format!(
        "AI call #{id}: {model}, version {version_id}{chart}, {status}, {} + {} tokens, {:.1} s, {requests}",
        trace.input_tokens,
        trace.output_tokens,
        trace.duration_ms as f64 / 1000.0,
    )
}

fn used_by(usage: &[(String, i64)], model: &str) -> i64 {
    usage
        .iter()
        .find(|(id, _)| id == model)
        .map(|(_, tokens)| *tokens)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_summary_reads_as_one_line() {
        let trace = CallTrace {
            input_tokens: 1500,
            output_tokens: 300,
            http_status: Some(200),
            attempts: 1,
            duration_ms: 12_400,
            ..CallTrace::default()
        };
        assert_eq!(
            call_summary(12, "claude-sonnet-5", 3, true, &trace),
            "AI call #12: claude-sonnet-5, version 3, with chart, HTTP 200, 1500 + 300 tokens, 12.4 s, 1 request"
        );
        let unreachable = CallTrace {
            attempts: 1,
            duration_ms: 90_000,
            ..CallTrace::default()
        };
        assert!(
            call_summary(13, "m", 3, false, &unreachable)
                .contains("m, version 3, no answer, 0 + 0 tokens, 90.0 s")
        );
    }
}
