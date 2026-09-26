use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{AssertSqlSafe, FromRow, SqlitePool};

use super::database::timestamp;
use crate::application::ports::WritingRepository;
use crate::domain::error::AppResult;
use crate::domain::writing::{
    Piece, PieceSummary, PieceUpdate, Version, VersionUpdate, WritingKind,
};

pub struct SqliteWritingRepository {
    pool: SqlitePool,
}

impl SqliteWritingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct PieceRow {
    id: i64,
    kind: String,
    prompt: String,
    written_on: NaiveDate,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct SummaryRow {
    id: i64,
    kind: String,
    prompt: String,
    written_on: NaiveDate,
    version_count: i64,
    latest_word_count: i64,
    latest_seconds_spent: Option<i64>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct VersionRow {
    id: i64,
    piece_id: i64,
    version_no: i64,
    body: String,
    word_count: i64,
    seconds_spent: Option<i64>,
    feedback: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// `From` lets us write `row.into()` to get a `Version`.
impl From<VersionRow> for Version {
    fn from(row: VersionRow) -> Self {
        Version {
            id: row.id,
            piece_id: row.piece_id,
            version_no: row.version_no,
            body: row.body,
            word_count: row.word_count,
            seconds_spent: row.seconds_spent,
            feedback: row.feedback,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

const SELECT_VERSION: &str = "
    SELECT id, piece_id, version_no, body, word_count, seconds_spent, feedback,
           created_at, updated_at
      FROM writing_versions";

#[async_trait]
impl WritingRepository for SqliteWritingRepository {
    async fn list(&self) -> AppResult<Vec<PieceSummary>> {
        let rows = sqlx::query_as::<_, SummaryRow>(
            "SELECT p.id, p.kind, p.prompt, p.written_on, p.updated_at,
                    (SELECT COUNT(*) FROM writing_versions v WHERE v.piece_id = p.id)
                        AS version_count,
                    COALESCE((SELECT v.word_count FROM writing_versions v
                               WHERE v.piece_id = p.id
                               ORDER BY v.version_no DESC LIMIT 1), 0)
                        AS latest_word_count,
                    (SELECT v.seconds_spent FROM writing_versions v
                      WHERE v.piece_id = p.id
                      ORDER BY v.version_no DESC LIMIT 1)
                        AS latest_seconds_spent
               FROM writing_pieces p
              ORDER BY p.written_on DESC, p.id DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(PieceSummary {
                    id: row.id,
                    kind: WritingKind::parse(&row.kind)?,
                    prompt: row.prompt,
                    written_on: row.written_on,
                    version_count: row.version_count,
                    latest_word_count: row.latest_word_count,
                    latest_seconds_spent: row.latest_seconds_spent,
                    updated_at: row.updated_at,
                })
            })
            .collect()
    }

    async fn get(&self, id: i64) -> AppResult<Option<Piece>> {
        let row = sqlx::query_as::<_, PieceRow>(
            "SELECT id, kind, prompt, written_on, created_at, updated_at
               FROM writing_pieces WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        // `let ... else`: if there is no row, return early with Ok(None).
        let Some(row) = row else {
            return Ok(None);
        };

        let sql = format!("{SELECT_VERSION} WHERE piece_id = ? ORDER BY version_no");
        let versions = sqlx::query_as::<_, VersionRow>(AssertSqlSafe(sql))
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

        Ok(Some(Piece {
            id: row.id,
            kind: WritingKind::parse(&row.kind)?,
            prompt: row.prompt,
            written_on: row.written_on,
            created_at: row.created_at,
            updated_at: row.updated_at,
            versions: versions.into_iter().map(Version::from).collect(),
        }))
    }

    async fn create(
        &self,
        kind: WritingKind,
        prompt: &str,
        written_on: NaiveDate,
        now: DateTime<Utc>,
    ) -> AppResult<i64> {
        let now = timestamp(now);
        let mut tx = self.pool.begin().await?;

        let id = sqlx::query(
            "INSERT INTO writing_pieces (kind, prompt, written_on, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(kind.as_str())
        .bind(prompt)
        .bind(written_on)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        sqlx::query(
            "INSERT INTO writing_versions (piece_id, version_no, created_at, updated_at)
             VALUES (?, 1, ?, ?)",
        )
        .bind(id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(id)
    }

    async fn update(&self, id: i64, update: &PieceUpdate, now: DateTime<Utc>) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE writing_pieces SET kind = ?, prompt = ?, written_on = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(update.kind.as_str())
        .bind(&update.prompt)
        .bind(update.written_on)
        .bind(timestamp(now))
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: i64) -> AppResult<bool> {
        // Versions are removed by ON DELETE CASCADE.
        let result = sqlx::query("DELETE FROM writing_pieces WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_version(&self, id: i64) -> AppResult<Option<Version>> {
        let sql = format!("{SELECT_VERSION} WHERE id = ?");
        let row = sqlx::query_as::<_, VersionRow>(AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Version::from))
    }

    async fn count_versions(&self, piece_id: i64) -> AppResult<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM writing_versions WHERE piece_id = ?",
        )
        .bind(piece_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    async fn add_version(&self, piece_id: i64, now: DateTime<Utc>) -> AppResult<Option<Version>> {
        let now = timestamp(now);
        let mut tx = self.pool.begin().await?;

        // Copy the latest version's body into a new row numbered one higher.
        // Timer and feedback start fresh.
        let result = sqlx::query(
            "INSERT INTO writing_versions
                (piece_id, version_no, body, word_count, created_at, updated_at)
             SELECT piece_id, version_no + 1, body, word_count, ?2, ?2
               FROM writing_versions
              WHERE piece_id = ?1
              ORDER BY version_no DESC
              LIMIT 1",
        )
        .bind(piece_id)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None); // piece doesn't exist
        }
        let version_id = result.last_insert_rowid();

        sqlx::query("UPDATE writing_pieces SET updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(piece_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        self.get_version(version_id).await
    }

    async fn update_version(
        &self,
        id: i64,
        update: &VersionUpdate,
        word_count: i64,
        now: DateTime<Utc>,
    ) -> AppResult<bool> {
        let now = timestamp(now);
        let mut tx = self.pool.begin().await?;

        // COALESCE(?, column): if the new value is NULL (field left out of
        // the request), keep the old one.
        let result = sqlx::query(
            "UPDATE writing_versions
                SET body = ?, word_count = ?,
                    seconds_spent = COALESCE(?, seconds_spent),
                    feedback = COALESCE(?, feedback),
                    updated_at = ?
              WHERE id = ?",
        )
        .bind(&update.body)
        .bind(word_count)
        .bind(update.seconds_spent)
        .bind(&update.feedback)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        // Keep the piece's updated_at fresh so "most recent piece" is right.
        sqlx::query(
            "UPDATE writing_pieces SET updated_at = ?
              WHERE id = (SELECT piece_id FROM writing_versions WHERE id = ?)",
        )
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    async fn delete_version(&self, id: i64) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM writing_versions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
