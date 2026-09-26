use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};

use super::database::timestamp;
use crate::application::ports::AiFeedbackRepository;
use crate::domain::error::{AppError, AppResult};
use crate::domain::feedback::{AiCall, AiFeedback, AiFeedbackRecord};

pub struct SqliteAiFeedbackRepository {
    pool: SqlitePool,
}

impl SqliteAiFeedbackRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct FeedbackRow {
    id: i64,
    version_id: i64,
    model: String,
    result_json: String,
    with_image: bool,
    input_tokens: i64,
    output_tokens: i64,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl AiFeedbackRepository for SqliteAiFeedbackRepository {
    async fn save(&self, call: AiCall<'_>) -> AppResult<i64> {
        let json = call
            .feedback
            .map(serde_json::to_string)
            .transpose()
            .map_err(|err| AppError::Internal(format!("feedback to json: {err}")))?;
        let trace = call.trace;
        // An empty request means nothing was sent; store NULL, not "".
        let request = Some(trace.request_json.as_str()).filter(|r| !r.is_empty());
        let id = sqlx::query(
            "INSERT INTO ai_feedback
                (version_id, model, result_json, with_image, input_tokens, output_tokens,
                 created_at, request_json, raw_reply, http_status, duration_ms, attempts, error)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(call.version_id)
        .bind(call.model)
        .bind(json)
        .bind(call.with_image)
        .bind(trace.input_tokens)
        .bind(trace.output_tokens)
        .bind(timestamp(call.created_at))
        .bind(request)
        .bind(trace.raw_reply.as_deref())
        .bind(trace.http_status)
        .bind(trace.duration_ms)
        .bind(trace.attempts)
        .bind(call.error)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    async fn for_piece(&self, piece_id: i64) -> AppResult<Vec<AiFeedbackRecord>> {
        let rows = sqlx::query_as::<_, FeedbackRow>(
            "SELECT a.id, a.version_id, a.model, a.result_json, a.with_image,
                    a.input_tokens, a.output_tokens, a.created_at
               FROM ai_feedback a JOIN writing_versions v ON v.id = a.version_id
              WHERE v.piece_id = ? AND a.result_json IS NOT NULL
              ORDER BY a.created_at DESC, a.id DESC",
        )
        .bind(piece_id)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::new();
        for row in rows {
            // We wrote this JSON ourselves, so a failure means a bug or a hand
            // edit; skip that one row instead of failing the whole page.
            match serde_json::from_str::<AiFeedback>(&row.result_json) {
                Ok(feedback) => records.push(AiFeedbackRecord {
                    id: row.id,
                    version_id: row.version_id,
                    model: row.model,
                    feedback,
                    with_image: row.with_image,
                    input_tokens: row.input_tokens,
                    output_tokens: row.output_tokens,
                    created_at: row.created_at,
                }),
                Err(err) => log::warn!("skipping ai_feedback {}: {err}", row.id),
            }
        }
        Ok(records)
    }

    async fn usage_since(&self, since: DateTime<Utc>) -> AppResult<Vec<(String, i64)>> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT model, SUM(input_tokens + output_tokens)
               FROM ai_feedback
              WHERE created_at >= ?
              GROUP BY model",
        )
        .bind(timestamp(since))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
