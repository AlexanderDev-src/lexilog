use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::application::ports::MistakeRepository;
use crate::domain::error::AppResult;
use crate::domain::mistakes::{
    MistakeCount, MistakeTagSummary, PeriodCount, PeriodWords, TrendBucket,
};

pub struct SqliteMistakeRepository {
    pool: SqlitePool,
}

impl SqliteMistakeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// strftime pattern for each bucket: "2026-09" or "2026-W38" (weeks start Monday).
fn period_format(bucket: TrendBucket) -> &'static str {
    match bucket {
        TrendBucket::Month => "%Y-%m",
        TrendBucket::Week => "%Y-W%W",
    }
}

#[async_trait]
impl MistakeRepository for SqliteMistakeRepository {
    async fn for_piece(&self, piece_id: i64) -> AppResult<Vec<MistakeCount>> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT t.name, pm.count
               FROM piece_mistakes pm JOIN mistake_tags t ON t.id = pm.tag_id
              WHERE pm.piece_id = ?
              ORDER BY pm.count DESC, t.name",
        )
        .bind(piece_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(tag, count)| MistakeCount { tag, count })
            .collect())
    }

    async fn replace_for_piece(&self, piece_id: i64, mistakes: &[MistakeCount]) -> AppResult<bool> {
        let mut tx = self.pool.begin().await?;

        let exists =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM writing_pieces WHERE id = ?")
                .bind(piece_id)
                .fetch_one(&mut *tx)
                .await?;
        if exists == 0 {
            return Ok(false);
        }

        sqlx::query("DELETE FROM piece_mistakes WHERE piece_id = ?")
            .bind(piece_id)
            .execute(&mut *tx)
            .await?;
        for mistake in mistakes {
            sqlx::query("INSERT INTO mistake_tags (name) VALUES (?) ON CONFLICT (name) DO NOTHING")
                .bind(&mistake.tag)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "INSERT INTO piece_mistakes (piece_id, tag_id, count)
                 SELECT ?, id, ? FROM mistake_tags WHERE name = ?",
            )
            .bind(piece_id)
            .bind(mistake.count)
            .bind(&mistake.tag)
            .execute(&mut *tx)
            .await?;
        }
        // Tags no piece uses any more would clutter autocomplete.
        sqlx::query("DELETE FROM mistake_tags WHERE id NOT IN (SELECT tag_id FROM piece_mistakes)")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    async fn tags(&self) -> AppResult<Vec<MistakeTagSummary>> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT t.name, COALESCE(SUM(pm.count), 0) AS total, COUNT(pm.piece_id)
               FROM mistake_tags t LEFT JOIN piece_mistakes pm ON pm.tag_id = t.id
              GROUP BY t.id
              ORDER BY total DESC, t.name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, total, pieces)| MistakeTagSummary {
                name,
                total,
                pieces,
            })
            .collect())
    }

    async fn trend_data(
        &self,
        bucket: TrendBucket,
    ) -> AppResult<(Vec<PeriodWords>, Vec<PeriodCount>)> {
        let format = period_format(bucket);

        // Only pieces that were actually checked count towards the words: a
        // piece nobody looked at has no tagged mistakes, and would make every
        // mistake look rarer than it is. "Checked" = it has mistake tags, or
        // any version has feedback (pasted or from the AI).
        let periods = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT strftime(?1, p.written_on) AS period,
                    COUNT(*) AS pieces,
                    COALESCE(SUM((SELECT v.word_count FROM writing_versions v
                                   WHERE v.piece_id = p.id
                                   ORDER BY v.version_no LIMIT 1)), 0) AS words
               FROM writing_pieces p
              WHERE EXISTS (SELECT 1 FROM piece_mistakes pm WHERE pm.piece_id = p.id)
                 OR EXISTS (SELECT 1 FROM writing_versions v
                             WHERE v.piece_id = p.id
                               AND (v.feedback <> ''
                                    OR EXISTS (SELECT 1 FROM ai_feedback a
                                                WHERE a.version_id = v.id
                                                  AND a.result_json IS NOT NULL)))
              GROUP BY period
              ORDER BY period",
        )
        .bind(format)
        .fetch_all(&self.pool)
        .await?;

        let counts = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT strftime(?1, p.written_on) AS period, t.name, SUM(pm.count)
               FROM piece_mistakes pm
               JOIN writing_pieces p ON p.id = pm.piece_id
               JOIN mistake_tags t ON t.id = pm.tag_id
              GROUP BY period, t.name",
        )
        .bind(format)
        .fetch_all(&self.pool)
        .await?;

        Ok((
            periods
                .into_iter()
                .map(|(period, pieces, words)| PeriodWords {
                    period,
                    pieces,
                    words,
                })
                .collect(),
            counts
                .into_iter()
                .map(|(period, tag, count)| PeriodCount { period, tag, count })
                .collect(),
        ))
    }
}
